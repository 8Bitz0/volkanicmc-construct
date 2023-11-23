use std::path;
use tokio::fs;
use tracing::debug;

const VKSTORE_PATH: &str = ".volkanic/";

const VKSTORE_BUILD_SUFFIX: &str = "build/";
const VKSTORE_DOWNLOADS_SUFFIX: &str = "downloads/";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Filesystem error: {0}")]
    FilesystemError(std::io::Error),
}

#[derive(Debug, Clone)]
pub struct VolkanicStore {
    pub path: path::PathBuf,
    pub build_path: path::PathBuf,
    pub downloads_path: path::PathBuf,
}

impl VolkanicStore {
    async fn create(&self) -> Result<(), StoreError> {
        debug!("Requested store creation at {:?}", self.path);

        if !&self.path.is_dir() {
            fs::create_dir_all(&self.path).await.map_err(StoreError::FilesystemError)?;
        } if !&self.build_path.is_dir() {
            fs::create_dir_all(&self.build_path).await.map_err(StoreError::FilesystemError)?;
        } if !&self.downloads_path.is_dir() {
            fs::create_dir_all(&self.downloads_path).await.map_err(StoreError::FilesystemError)?;
        }

        Ok(())
    }
    pub async fn init() -> Result<Self, StoreError> {
        let store = Self {
            path: path::PathBuf::from(VKSTORE_PATH),
            build_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_BUILD_SUFFIX),
            downloads_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_DOWNLOADS_SUFFIX),
        };

        store.create().await?;

        Ok(store)
    }
}
