use std::path;
use tokio::{fs, task::spawn_blocking};
use tracing::{debug, error};

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

async fn clear_dir<T: AsRef<path::Path>>(path: T) -> tokio::io::Result<()> {
    let path = path.as_ref().to_path_buf();

    debug!("Clearing directory: \"{}\"", path.to_string_lossy());

    for e in spawn_blocking(move || std::fs::read_dir(path)).await?? {
        let p = e?.path();

        if p.is_file() {
            debug!("Removing inner file: \"{}\"", p.to_string_lossy());

            fs::remove_file(p).await?;
        } else if p.is_dir() {
            debug!("Removing inner directory: \"{}\"", p.to_string_lossy());

            fs::remove_dir_all(p).await?;
        } else {
            error!("Directory \"{}\" not found", p.to_string_lossy());
        }
    }

    Ok(())
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
    /// Creates a new `VolkanicStore`
    pub async fn new() -> Self {
        Self {
            path: path::PathBuf::from(VKSTORE_PATH),
            build_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_BUILD_SUFFIX),
            downloads_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_DOWNLOADS_SUFFIX),
            runtime_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_RUNTIME_SUFFIX),
            temp_path: path::PathBuf::from(VKSTORE_PATH).join(VKSTORE_TEMP_SUFFIX),
        }
    }
    /// Creates a new `VolkanicStore` with a custom root directory
    pub async fn new_custom_root<P: AsRef<path::Path>>(root_path: P) -> Self {
        let root = root_path.as_ref().to_path_buf();

        Self {
            path: root.to_path_buf().clone(),
            build_path: root.to_path_buf().clone().join(VKSTORE_BUILD_SUFFIX),
            downloads_path: root.to_path_buf().clone().join(VKSTORE_DOWNLOADS_SUFFIX),
            runtime_path: root.to_path_buf().clone().join(VKSTORE_RUNTIME_SUFFIX),
            temp_path: root.to_path_buf().clone().join(VKSTORE_TEMP_SUFFIX),
        }
    }
    /// Changes the build directory for the store
    pub async fn override_build<P: AsRef<path::Path>>(&self, path: P) -> Self {
        let mut store = self.clone();

        store.build_path = path.as_ref().to_path_buf();

        store
    }
    /// Changes the downloads directory for the store
    pub async fn override_downloads<P: AsRef<path::Path>>(&self, path: P) -> Self {
        let mut store = self.clone();

        store.downloads_path = path.as_ref().to_path_buf();

        store
    }
    /// Create directories for store
    pub async fn init(&self) -> Result<(), StoreError> {
        self.create().await?;

        Ok(())
    }
    /// Removes temporary files which shouldn't persist across runs
    pub async fn clean(&self) -> Result<(), StoreError> {
        let to_remove = [&self.temp_path];

        for dir in to_remove {
            clear_dir(dir).await.map_err(StoreError::Filesystem)?;
        }

        Ok(())
    }
    /// Removes all downloaded files
    pub async fn clear_downloads(&self) -> Result<(), StoreError> {
        let to_remove = [&self.downloads_path, &self.temp_path];

        for dir in to_remove {
            if dir.is_dir() {
                clear_dir(dir).await.map_err(StoreError::Filesystem)?;
            }
        }

        Ok(())
    }
    /// Removes all build and runtime files
    pub async fn renew(&self) -> Result<(), StoreError> {
        let to_clear = [&self.build_path, &self.runtime_path];

        for dir in to_clear {
            if dir.is_dir() {
                clear_dir(dir).await.map_err(StoreError::Filesystem)?;
            }
        }

        Ok(())
    }
}
