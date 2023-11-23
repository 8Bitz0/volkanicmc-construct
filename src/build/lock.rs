use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info};

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
    pub jobs: Vec<job::Job>,
    pub job_progress: usize,
}

impl Lock {
    pub async fn new(store: &vkstore::VolkanicStore, jobs: Vec<job::Job>) -> Result<Lock, LockError> {
        let lock = Lock {
            jobs,
            job_progress: 0,
        };
        
        lock.update(store).await?;

        info!("Directory locked.");

        Ok(lock)
    }
    pub async fn update(&self, store: &vkstore::VolkanicStore) -> Result<(), LockError> {
        let mut f = fs::File::create(store.path.join(LOCK_SUFFIX)).await.map_err(
            LockError::FilesystemError,
        )?;
        
        f.write_all(serde_jsonc::to_string_pretty(&self).map_err(LockError::JsonSerializeError)?.as_bytes())
            .await
            .map_err(LockError::FilesystemError)?;

        debug!("Lock updated.");

        Ok(())
    }
    pub async fn remove(&self, store: &vkstore::VolkanicStore) -> Result<(), LockError> {
        fs::remove_file(store.build_path.join(LOCK_SUFFIX)).await
            .map_err(LockError::FilesystemError)?;

        info!("Directory unlocked.");
        
        Ok(())
    }
}
