use std::path;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::resources::{self, Jdk};
use crate::vkstore;

use super::misc::{
    download_indicatif, extract, get_remote_filename, DownloadError, ExtractionError, Verification,
};

#[derive(Debug, thiserror::Error)]
pub enum PrepareJdkError {
    #[error("Download error: {0}")]
    Download(DownloadError),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
    #[error("Extraction error: {0}")]
    Extraction(ExtractionError),
    #[error("Invalid JDK home directory: {0}")]
    InvalidJdkHome(path::PathBuf),
    #[error("Directory failed to be copied: {0}")]
    DirectoryCopyFailed(path::PathBuf),
    #[error("No name for URL: {0}")]
    NoNameForUrl(String),
}

pub async fn prepare_jdk(store: vkstore::VolkanicStore, jdk: Jdk) -> Result<(), PrepareJdkError> {
    let jdk_name = match get_remote_filename(&jdk.url).await {
        Some(s) => s,
        None => return Err(PrepareJdkError::NoNameForUrl(jdk.url.clone())),
    };

    download_indicatif(
        store.clone(),
        &jdk.url,
        Verification::Sha256(jdk.sha256),
        jdk_name.clone(),
        None,
    )
    .await
    .map_err(PrepareJdkError::Download)?;

    let p = store.downloads_path.join(jdk_name);
    let ex_path = extract(store.clone(), p, jdk.format)
        .await
        .map_err(PrepareJdkError::Extraction)?;

    if !ex_path
        .join(&jdk.home_path)
        .join(resources::conf::JDK_BIN_FILE)
        .is_file()
    {
        return Err(PrepareJdkError::InvalidJdkHome(ex_path.join(jdk.home_path)));
    }

    if store.runtime_path.is_dir() {
        warn!("Removing existing runtime directory");
        fs::remove_dir_all(&store.runtime_path)
            .await
            .map_err(PrepareJdkError::Filesystem)?;
    }

    match copy_dir::copy_dir(ex_path.join(&jdk.home_path), &store.runtime_path) {
        Ok(_) => {
            info!("Copied JDK to runtime directory");
        }
        Err(e) => {
            debug!("Errors ocurred during JDK copy: {:#?}", e);
            return Err(PrepareJdkError::DirectoryCopyFailed(
                ex_path.join(jdk.home_path),
            ));
        }
    }

    Ok(())
}
