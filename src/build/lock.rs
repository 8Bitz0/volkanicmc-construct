use serde::{Deserialize, Serialize};
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info, warn, error};

use crate::vkstore;

use super::job;

const LOCK_SUFFIX: &str = "lock.json";

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(serde_jsonc::Error),
    #[error("Failed to serialize JSON: {0}")]
    JsonSerializeError(serde_jsonc::Error),
    #[error("Filesystem error: {0}")]
    FilesystemError(tokio::io::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lock {
    #[serde(skip)]
    path: Option<path::PathBuf>,
    #[serde(skip, default)]
    pub stray: bool,
    pub jobs: Vec<job::Job>,
    pub job_progress: usize,
}

impl Default for Lock {
    fn default() -> Self {
        Self {
            path: None,
            stray: false,
            jobs: vec![],
            job_progress: 0,
        }
    }
}

impl Lock {
    pub async fn new(store: &vkstore::VolkanicStore, jobs: Vec<job::Job>) -> Result<Lock, LockError> {
        let lock_path = store.path.join(LOCK_SUFFIX);

        let mut lock = Lock {
            jobs,
            stray: false,
            job_progress: 0,
            path: Some(lock_path.clone()),
        };

        if lock_path.is_file() {
            let f_contents = fs::read_to_string(&lock_path).await.map_err(
                LockError::FilesystemError,
            )?;

            let mut stray_lock = serde_jsonc::from_str::<Lock>(&f_contents).map_err(LockError::JsonParseError)?;
            stray_lock.path = Some(lock_path);
            stray_lock.stray = true;

            if stray_lock.jobs == lock.jobs {
                warn!("Stray lock was found, and complies with current template. Build will continue...");
                lock = stray_lock;
            } else {
                warn!("Stray lock was found, but doesn't seem to follow the current template. Build will be reset.")
            }
        }
        
        lock.update().await?;

        info!("Directory locked");

        Ok(lock)
    }
    pub async fn update(&self) -> Result<(), LockError> {
        match &self.path {
            Some(path) => {
                let mut f = fs::File::create(&path).await.map_err(
                    LockError::FilesystemError,
                )?;
                
                match f.write_all(match serde_jsonc::to_string_pretty(&self) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to serialize lock file: {}", e);
                        return Err(LockError::JsonSerializeError(e));
                    }
                }.as_bytes()).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to write lock file: {}", e);
                        return Err(LockError::FilesystemError(e));
                    }
                };
        
                debug!("Lock updated");
            }
            None => {
                warn!("No lock file path was specified. Lock was not updated.");
            }
        }
        

        Ok(())
    }
    pub async fn remove(&self) -> Result<(), LockError> {
        match &self.path {
            Some(path) => {
                match fs::remove_file(path).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to remove lock file: {}", e);
                        return Err(LockError::FilesystemError(e));
                    }
                }
            }
            None => {
                warn!("No lock file path was specified. Lock was not removed.");
            }
        }

        info!("Directory unlocked");

        Ok(())
    }
}
