use std::path;
use tokio::fs;
use tracing::{debug, info};

const VKSTORE_PATH: &str = ".volkanic/";

const VKSTORE_BUILD_SUFFIX: &str = "build/";
const VKSTORE_DOWNLOADS_SUFFIX: &str = "downloads/";
const VKSTORE_RUNTIME_SUFFIX: &str = "runtime/";
const VKSTORE_TEMP_SUFFIX: &str = "temp/";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Filesystem error: {0}")]
    Filesystem(std::io::Error),
}

#[derive(Debug, Clone)]
pub struct VolkanicStore {
    pub path: path::PathBuf,
    pub build_path: path::PathBuf,
    pub downloads_path: path::PathBuf,
    pub runtime_path: path::PathBuf,
    // TODO: Create temporary file management
    pub temp_path: path::PathBuf,
}

impl VolkanicStore {
    async fn create(&self) -> Result<(), StoreError> {
        debug!("Requested store creation at {:?}", self.path);

        let to_create = [
            &self.path,
            &self.build_path,
            &self.downloads_path,
            &self.temp_path,
        ];

        for p in to_create {
            if !p.is_dir() {
                debug!("Creating Volkanic folder: \"{}\"", p.to_string_lossy());
                fs::create_dir_all(p)
                    .await
                    .map_err(StoreError::Filesystem)?;
            } else {
                debug!(
                    "Volkanic folder already exists: \"{}\"",
                    p.to_string_lossy()
                );
            }
        }

        Ok(())
    }
    pub async fn exists() -> bool {
        path::Path::new(VKSTORE_PATH).is_dir()
    }
    /// Creates a new `VolkanicStore` and creates all necessary subdirectories
    pub async fn init<T: AsRef<path::Path>>(override_build: Option<T>) -> Result<Self, StoreError> {
        let store = Self {
            path: path::PathBuf::from(VKSTORE_PATH),
            build_path: match override_build {
                Some(p) => {
                    info!(
                        "Overriding build directory, using: \"{}\"",
                        p.as_ref().to_string_lossy()
                    );

                    p.as_ref().to_path_buf()
                }
                None => path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_BUILD_SUFFIX),
            },
            downloads_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_DOWNLOADS_SUFFIX),
            runtime_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_RUNTIME_SUFFIX),
            temp_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_TEMP_SUFFIX),
        };

        store.create().await?;

        Ok(store)
    }
    /// Removes temporary files which shouldn't persist across runs
    pub async fn clean(&self) -> Result<(), StoreError> {
        let to_remove = [&self.temp_path];

        for dir in to_remove {
            fs::remove_dir_all(dir)
                .await
                .map_err(StoreError::Filesystem)?;
        }

        Ok(())
    }
    /// Removes all downloaded files
    pub async fn clear_downloads(&self) -> Result<(), StoreError> {
        let to_remove = [&self.downloads_path, &self.temp_path];

        for dir in to_remove {
            if dir.is_dir() {
                fs::remove_dir_all(dir)
                    .await
                    .map_err(StoreError::Filesystem)?;
            }
        }

        Ok(())
    }
    /// Removes all build and runtime files
    pub async fn renew(&self) -> Result<(), StoreError> {
        let to_clear = [&self.build_path, &self.runtime_path];

        for dir in to_clear {
            if dir.is_dir() {
                fs::remove_dir_all(dir)
                    .await
                    .map_err(StoreError::Filesystem)?;
                fs::create_dir_all(dir)
                    .await
                    .map_err(StoreError::Filesystem)?;
            }
        }

        Ok(())
    }
}
