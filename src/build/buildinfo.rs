use serde::{Deserialize, Serialize};
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info, warn, error};

use crate::vkstore;

use super::job;

const BUILD_INFO_SUFFIX: &str = "build.json";

#[derive(Debug, thiserror::Error)]
pub enum BuildInfoError {
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(serde_jsonc::Error),
    #[error("Failed to serialize JSON: {0}")]
    JsonSerializeError(serde_jsonc::Error),
    #[error("Filesystem error: {0}")]
    FilesystemError(tokio::io::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildInfo {
    #[serde(skip)]
    path: Option<path::PathBuf>,
    #[serde(skip, default)]
    pub stray: bool,
    pub jobs: Vec<job::Job>,
    pub job_progress: usize,
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            path: None,
            stray: false,
            jobs: vec![],
            job_progress: 0,
        }
    }
}

impl BuildInfo {
    pub async fn exists(store: &vkstore::VolkanicStore) -> bool {
        store.path.join(BUILD_INFO_SUFFIX).is_file()
    }
    pub async fn get(store: &vkstore::VolkanicStore) -> Result<BuildInfo, BuildInfoError> {
        let build_info_path = store.path.join(BUILD_INFO_SUFFIX);

        let f_contents = fs::read_to_string(&build_info_path).await.map_err(
            BuildInfoError::FilesystemError,
        )?;

        Ok(serde_jsonc::from_str::<BuildInfo>(&f_contents).map_err(BuildInfoError::JsonParseError)?)
    }
    pub async fn new(store: &vkstore::VolkanicStore) -> Result<BuildInfo, BuildInfoError> {
        let mut build_info = BuildInfo {
            jobs: vec![],
            stray: false,
            job_progress: 0,
            path: None,
        };

        build_info.set_path(store);
        
        build_info.update().await?;

        info!("Directory build info created");

        Ok(build_info)
    }
    pub async fn update(&self) -> Result<(), BuildInfoError> {
        match &self.path {
            Some(path) => {
                let mut f = fs::File::create(&path).await.map_err(
                    BuildInfoError::FilesystemError,
                )?;
                
                match f.write_all(match serde_jsonc::to_string_pretty(&self) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to serialize build info file: {}", e);
                        return Err(BuildInfoError::JsonSerializeError(e));
                    }
                }.as_bytes()).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to write build info file: {}", e);
                        return Err(BuildInfoError::FilesystemError(e));
                    }
                };
        
                debug!("Build info updated");
            }
            None => {
                warn!("No build info file path was specified. Build info not updated.");
            }
        }
        

        Ok(())
    }
    pub fn set_path(&mut self, store: &vkstore::VolkanicStore) {
        self.path = Some(store.path.join(BUILD_INFO_SUFFIX));
    }
    pub async fn remove(&self) -> Result<(), BuildInfoError> {
        match &self.path {
            Some(path) => {
                match fs::remove_file(path).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to remove build info file: {}", e);
                        return Err(BuildInfoError::FilesystemError(e));
                    }
                }
            }
            None => {
                warn!("No build info file path was specified. Build info was not removed.");
            }
        }

        info!("Build info removed");

        Ok(())
    }
}