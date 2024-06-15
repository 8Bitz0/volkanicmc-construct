use flate2::read::GzDecoder;
use futures_util::stream::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::path;
use tokio::{
    fs,
    io::{self, AsyncReadExt, AsyncWriteExt},
};
use tracing::{error, info, warn};

use crate::{
    resources::{self, style, ArchiveFormat},
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

pub async fn get_remote_filename<T: std::fmt::Display>(url: T) -> Option<String> {
    let str = url.to_string();

    let split_url = str.split('/').collect::<Vec<&str>>();

    if split_url.len() < 2 {
        None
    } else {
        Some(split_url[split_url.len() - 1].to_string())
    }
}

pub async fn default_user_agent() -> String {
    format!("8Bitz0/volkanicmc/{}", env!("CARGO_PKG_VERSION"))
}

pub async fn download_progress<U: std::fmt::Display, N: std::fmt::Display, A: std::fmt::Display>(
    store: vkstore::VolkanicStore,
    url: U,
    verification: Verification,
    name: N,
    user_agent: Option<A>,
) -> Result<path::PathBuf, DownloadError> {
    let p = store.downloads_path.join(match &verification {
        Verification::None => format!("noverify-{}", &name),
        Verification::Sha256(sha256) => sha256.to_string(),
        Verification::Sha512(sha512) => sha512.to_string(),
    });

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

    let user_agent = match user_agent {
        Some(u) => u.to_string(),
        None => default_user_agent().await,
    };

    let response = client
        .get(url.to_string())
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await
        .map_err(DownloadError::Http)?;

    let content_length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(content_length);
    pb.set_style(style::get_pb_style(style::ProgressStyleType::Bytes).await);

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
