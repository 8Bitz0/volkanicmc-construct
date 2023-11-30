use serde::{Deserialize, Serialize};
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info, warn, error};

use crate::exec;
use crate::hostinfo;
use crate::vkstore;

use super::job;

const BUILD_INFO_SUFFIX: &str = "build.json";

#[derive(Debug, thiserror::Error)]
pub enum BuildInfoError {
    #[error("Failed to parse JSON: {0}")]
    JsonParse(serde_jsonc::Error),
    #[error("Failed to serialize JSON: {0}")]
    JsonSerialize(serde_jsonc::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildInfo {
    #[serde(skip)]
    path: Option<path::PathBuf>,
    #[serde(skip, default)]
    pub stray: bool,
    pub jobs: Vec<job::Job>,
    #[serde(rename = "job-progress")]
    pub job_progress: usize,
    pub exec: Option<exec::BuildExecInfo>,
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            path: None,
            stray: false,
            jobs: vec![],
            job_progress: 0,
            exec: None,
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
            BuildInfoError::Filesystem,
        )?;

        Ok(serde_jsonc::from_str::<BuildInfo>(&f_contents).map_err(BuildInfoError::JsonParse)?)
    }
    pub async fn new(store: &vkstore::VolkanicStore) -> Result<BuildInfo, BuildInfoError> {
        let mut build_info = BuildInfo::default();

        build_info.set_path(store);
        
        build_info.update().await?;

        info!("Directory build info created");

        Ok(build_info)
    }
    pub async fn update(&self) -> Result<(), BuildInfoError> {
        match &self.path {
            Some(path) => {
                let mut f = fs::File::create(&path).await.map_err(
                    BuildInfoError::Filesystem,
                )?;
                
                match f.write_all(match serde_jsonc::to_string_pretty(&self) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to serialize build info file: {}", e);
                        return Err(BuildInfoError::JsonSerialize(e));
                    }
                }.as_bytes()).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to write build info file: {}", e);
                        return Err(BuildInfoError::Filesystem(e));
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
                        return Err(BuildInfoError::Filesystem(e));
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
