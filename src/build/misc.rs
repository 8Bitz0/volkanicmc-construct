use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::path;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::vkstore;

const FILE_BUFFER_SIZE: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    HttpError(reqwest::Error),
    #[error("Filesystem error: {0}")]
    FilesystemError(io::Error),
    #[error("Hex error: {0}")]
    HexError(hex::FromHexError),
}

pub enum Verification {
    Sha256(String),
    Sha512(String),
}

pub async fn download(store: vkstore::VolkanicStore, url: &str, target_path: path::PathBuf) -> Result<(), DownloadError> {
    let client = Client::new();
    
    let response = client.get(url).send().await.map_err(DownloadError::HttpError)?;

    let content_length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(content_length);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
        .progress_chars("#>-"));

    let mut dest = File::create(store.downloads_path.join(target_path)).await.map_err(DownloadError::FilesystemError)?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(DownloadError::HttpError)?;
        pb.inc(chunk.len() as u64);

        dest.write_all(&chunk).await.map_err(DownloadError::FilesystemError)?;
    }

    pb.finish_with_message("Download complete");

    Ok(())
}

pub async fn verify_hash(store: vkstore::VolkanicStore, target_path: path::PathBuf, verification: Verification) -> Result<bool, DownloadError> {
    match verification {
        Verification::Sha256(checksum) => {
            let mut hasher = Sha256::new();
            let mut file = File::open(store.downloads_path.join(target_path)).await.map_err(DownloadError::FilesystemError)?;

            let mut buffer = [0; FILE_BUFFER_SIZE];

            loop {
                let bytes_read = file.read(&mut buffer).await.map_err(DownloadError::FilesystemError)?;

                if bytes_read == 0 {
                    break;
                }

                hasher.update(&buffer[..bytes_read]);
            }
            
            Ok(hasher.finalize()[..] == hex::decode(checksum).map_err(DownloadError::HexError)?)
        }
        Verification::Sha512(checksum) => {
            let mut hasher = Sha512::new();
            let mut file = File::open(store.downloads_path.join(target_path)).await.map_err(DownloadError::FilesystemError)?;

            let mut buffer = [0; FILE_BUFFER_SIZE];

            loop {
                let bytes_read = file.read(&mut buffer).await.map_err(DownloadError::FilesystemError)?;

                if bytes_read == 0 {
                    break;
                }

                hasher.update(&buffer[..bytes_read]);
            }
            
            Ok(hasher.finalize()[..] == hex::decode(checksum).map_err(DownloadError::HexError)?)
        }
    }
}