use std::path;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::resources::Jdk;
use crate::vkstore;

use super::misc::{
    download, extract, get_remote_filename, DownloadError, ExtractionError, Verification,
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
}

pub async fn prepare_jdk(store: vkstore::VolkanicStore, jdk: Jdk) -> Result<(), PrepareJdkError> {
    let jdk_name = get_remote_filename(&jdk.url).await;

    download(
        store.clone(),
        &jdk.url,
        Verification::Sha256(jdk.sha256),
        jdk_name.into(),
    )
    .await
    .map_err(PrepareJdkError::Download)?;

    let p = store
        .downloads_path
        .join(get_remote_filename(&jdk.url).await);
    let ex_path = extract(store.clone(), p, jdk.format)
        .await
        .map_err(PrepareJdkError::Extraction)?;

    if !ex_path
        .join(&jdk.home_path)
        .join("bin")
        .join("java")
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
