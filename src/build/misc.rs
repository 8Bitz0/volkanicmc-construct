use flate2::read::GzDecoder;
use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::path;
use tokio::{
    fs,
    io::{self, AsyncReadExt, AsyncWriteExt},
};
use tracing::{debug, error, info, warn};

use crate::{
    resources::{self, ArchiveFormat},
    vkstore,
};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(io::Error),
    #[error("Hex error: {0}")]
    Hex(hex::FromHexError),
    #[error("Directory already exists: {0}")]
    DirectoryAlreadyExists(path::PathBuf),
    #[error("Verification failure: {0}")]
    VerificationFailure(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verification {
    None,
    Sha256(String),
    Sha512(String),
}

pub async fn get_remote_filename(url: &str) -> Option<String> {
    let split_url = url.split('/').collect::<Vec<&str>>();

    if split_url.len() < 2 {
        None
    } else {
        Some(split_url[split_url.len() - 1].to_string())
    }
}

pub async fn download_indicatif(
    store: vkstore::VolkanicStore,
    url: &str,
    verification: Verification,
    name: String,
) -> Result<path::PathBuf, DownloadError> {
    let p = store.downloads_path.join(&name);

    if p.is_dir() {
        return Err(DownloadError::DirectoryAlreadyExists(p));
    }

    if p.is_file() {
        if verify_hash(p.clone(), &verification).await? {
            return Ok(p);
        } else {
            warn!("Previously downloaded file for \"{}\" was unable to verify. The file will be re-downloaded.", name);
        }
    }

    info!("Downloading \"{}\"...", name);

    let client = Client::new();

    let response = client.get(url).send().await.map_err(DownloadError::Http)?;

    let content_length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(content_length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.green/white}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#/-"),
    );

    let mut dest = match fs::File::create(&p).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to create file \"{}\": {}", p.to_string_lossy(), e);
            return Err(DownloadError::Filesystem(e));
        }
    };

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(DownloadError::Http)?;
        pb.inc(chunk.len() as u64);

        match dest.write_all(&chunk).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to write to file \"{}\": {}", p.to_string_lossy(), e);
                return Err(DownloadError::Filesystem(e));
            }
        }
    }

    pb.finish();

    if p.is_file() {
        if verify_hash(p.clone(), &verification).await? {
            return Ok(p);
        } else {
            error!("Downloaded file for \"{}\" was unable to verify. This could be an issue with the template, or somebody is doing something nasty.", name);
            return Err(DownloadError::VerificationFailure(url.to_string()));
        }
    }

    Ok(p)
}

pub async fn verify_hash(
    target_path: path::PathBuf,
    verification: &Verification,
) -> Result<bool, DownloadError> {
    if verification != &Verification::None {
        info!("Verifying \"{}\"...", target_path.to_string_lossy());
    }

    let mut file = match fs::File::open(target_path.clone()).await {
        Ok(f) => f,
        Err(e) => {
            error!(
                "Failed to open file \"{}\": {}",
                target_path.to_string_lossy(),
                e
            );
            return Err(DownloadError::Filesystem(e));
        }
    };

    let mut buffer = [0; resources::conf::FILE_BUFFER_SIZE];

    match verification {
        Verification::Sha256(checksum) => {
            let mut hasher = Sha256::new();

            loop {
                let bytes_read = file
                    .read(&mut buffer)
                    .await
                    .map_err(DownloadError::Filesystem)?;

                if bytes_read == 0 {
                    break;
                }

                hasher.update(&buffer[..bytes_read]);
            }

            Ok(hasher.finalize()[..] == hex::decode(checksum).map_err(DownloadError::Hex)?)
        }
        Verification::Sha512(checksum) => {
            let mut hasher = Sha512::new();

            loop {
                let bytes_read = file
                    .read(&mut buffer)
                    .await
                    .map_err(DownloadError::Filesystem)?;

                if bytes_read == 0 {
                    break;
                }

                hasher.update(&buffer[..bytes_read]);
            }

            Ok(hasher.finalize()[..] == hex::decode(checksum).map_err(DownloadError::Hex)?)
        }
        Verification::None => Ok(true),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("Filesystem error: {0}")]
    FilesystemError(io::Error),
    #[error("No file name for path: {0}")]
    NoFileName(path::PathBuf),
    #[error("Tar archive error: {0}")]
    TarError(std::io::Error),
    #[error("Zip archive error: {0}")]
    ZipError(zip::result::ZipError),
}

pub async fn extract(
    store: vkstore::VolkanicStore,
    orig_path: path::PathBuf,
    format: ArchiveFormat,
) -> Result<path::PathBuf, ExtractionError> {
    let new_path = store.temp_path.join(match orig_path.file_name() {
        Some(file_name) => file_name,
        None => return Err(ExtractionError::NoFileName(orig_path)),
    });

    if new_path.is_dir() {
        fs::remove_dir_all(&new_path)
            .await
            .map_err(ExtractionError::FilesystemError)?;
    }

    info!("Extracting \"{}\"...", orig_path.to_string_lossy());

    match format {
        ArchiveFormat::TarGz => {
            let f = std::fs::File::open(&orig_path).map_err(ExtractionError::FilesystemError)?;

            let gz_decoder = GzDecoder::new(f);
            let mut archive = tar::Archive::new(gz_decoder);

            fs::create_dir_all(&new_path)
                .await
                .map_err(ExtractionError::FilesystemError)?;

            archive
                .unpack(&new_path)
                .map_err(ExtractionError::TarError)?;
        }
        ArchiveFormat::Zip => {
            let f = std::fs::File::open(&orig_path).map_err(ExtractionError::FilesystemError)?;

            let mut archive = zip::ZipArchive::new(f).map_err(ExtractionError::ZipError)?;

            fs::create_dir_all(&new_path)
                .await
                .map_err(ExtractionError::FilesystemError)?;

            archive
                .extract(&new_path)
                .map_err(ExtractionError::ZipError)?;
        }
    }

    Ok(new_path)
}

#[derive(Debug, PartialEq)]
pub enum FsObjectType {
    None,
    File,
    Directory,
}

impl std::fmt::Display for FsObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn fs_obj(path: path::PathBuf) -> FsObjectType {
    if path.is_file() {
        FsObjectType::File
    } else if path.is_dir() {
        FsObjectType::Directory
    } else {
        FsObjectType::None
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CreateAncestorError {
    #[error("Filesystem error: {0}")]
    FilesystemError(io::Error),
    #[error("Found wrong filesystem object at: \"{0}\" (expected: {1}, found: {2})")]
    WrongFsObject(path::PathBuf, FsObjectType, FsObjectType),
    #[error("No parent directory for path: {0}")]
    NoParentDir(path::PathBuf),
}

pub async fn create_ancestors(path: path::PathBuf) -> Result<(), CreateAncestorError> {
    debug!("Creating ancestors for \"{}\"", path.to_string_lossy());
    if let Some(parent) = path.clone().parent() {
        debug!("Direct parent path: \"{}\"", parent.to_string_lossy());
        match fs_obj(path.clone()) {
            FsObjectType::Directory => {
                debug!("Ancestors already exist for \"{}\"", path.to_string_lossy());
                Ok(())
            }
            FsObjectType::None => {
                fs::create_dir_all(parent)
                    .await
                    .map_err(CreateAncestorError::FilesystemError)?;

                Ok(())
            }
            _ => {
                error!("Wrong filesystem object at: \"{}\"", path.to_string_lossy());
                Err(CreateAncestorError::WrongFsObject(
                    path.clone(),
                    FsObjectType::Directory,
                    fs_obj(path),
                ))
            }
        }
    } else {
        Err(CreateAncestorError::NoParentDir(path))
    }
}
