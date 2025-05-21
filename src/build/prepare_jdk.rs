use std::path;
use tokio::fs;
use tracing::{debug, error, info};

use crate::resources::{self, HomePathType, Jdk};
use crate::vkstore;

use super::misc::{
    download_progress, extract, get_remote_filename, DownloadError, ExtractionError, Verification,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Download error: {0}")]
    Download(DownloadError),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
    #[error("Extraction error: {0}")]
    Extraction(ExtractionError),
    #[error("Invalid JDK home directory: {0}")]
    InvalidJdkHome(path::PathBuf),
    #[error("Failed to detect JDK home directory")]
    JdkHomeUndetected,
    #[error("Directory failed to be copied: {0}")]
    DirectoryCopyFailed(path::PathBuf),
    #[error("No name for URL: {0}")]
    NoNameForUrl(String),
}

pub async fn prepare_jdk(
    store: vkstore::VolkanicStore,
    jdk: Jdk,
    no_verify: bool,
) -> Result<(), Error> {
    let jdk_name = match get_remote_filename(&jdk.url).await {
        Some(s) => s,
        None => return Err(Error::NoNameForUrl(jdk.url.clone())),
    };

    let jdk_path = download_progress(
        store.clone(),
        &jdk.url,
        // Ignore verification parameters if the no verify flag is enabled
        if no_verify {
            Verification::None
        } else if let Some(sha256) = jdk.sha256 {
            Verification::Sha256(sha256)
        } else {
            Verification::None
        },
        &jdk_name,
        None::<String>,
    )
    .await
    .map_err(Error::Download)?;

    let ex_path = extract(store.clone(), jdk_path, jdk.format)
        .await
        .map_err(Error::Extraction)?;

    let home_path = match jdk.home_path {
        HomePathType::Custom(p) => p,
        HomePathType::FirstSubDir => {
            let dir = match ex_path.read_dir() {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to read directory: {e}");
                    return Err(Error::Filesystem(e));
                }
            };

            let mut found_name = None;
            for e in dir {
                let e = e.unwrap();
                let filename = e.file_name();
                let file_type = e.file_type().unwrap();
                if file_type.is_dir() {
                    found_name = Some(filename.into_string().unwrap());
                    break;
                }
            }
            
            found_name.ok_or(Error::JdkHomeUndetected)?
        }
    };

    if !ex_path
        .join(&home_path)
        .join(resources::conf::JDK_BIN_FILE)
        .is_file()
    {
        return Err(Error::InvalidJdkHome(ex_path.join(home_path)));
    }

    if store.runtime_path.is_dir() {
        info!("Removing existing runtime directory");
        fs::remove_dir_all(&store.runtime_path)
            .await
            .map_err(Error::Filesystem)?;
    }

    match copy_dir::copy_dir(ex_path.join(&home_path), &store.runtime_path) {
        Ok(_) => {
            info!("Copied JDK to runtime directory");
        }
        Err(e) => {
            debug!("Errors ocurred during JDK copy: {:#?}", e);
            return Err(Error::DirectoryCopyFailed(
                ex_path.join(home_path),
            ));
        }
    }

    Ok(())
}
