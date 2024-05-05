use copy_dir::copy_dir;
use std::path;
use tokio::{fs, task::spawn_blocking};
use tracing::{debug, error, info};

use crate::{
    misc::{fs_obj, FsObjectType},
    vkstore,
};

use super::PersistentObject;

#[derive(Debug, thiserror::Error)]
pub enum CopyPersistentError {
    #[error("Filesystem error {0}")]
    Filesystem(std::io::Error),
    #[error("Directory copy failed: {0}")]
    DirectoryCopyFailed(path::PathBuf),
    #[error("Directory already exists: {0}")]
    DirectoryAlreadyExists(path::PathBuf),
    #[error("File already exists: {0}")]
    FileAlreadyExists(path::PathBuf),
}

pub async fn save_persistent(
    store: &vkstore::VolkanicStore,
    persistent: Vec<PersistentObject>,
    new_path: path::PathBuf,
) -> Result<(), CopyPersistentError> {
    for p in persistent {
        match p {
            PersistentObject::Directory(inner_path) => {
                let current_path = store.build_path.join(inner_path.clone());
                let new_path = new_path.join(&inner_path);

                if fs_obj(&current_path).await != FsObjectType::Directory {
                    error!(
                        "No directory for persistent object: {}",
                        inner_path.to_string_lossy()
                    );
                    continue;
                }

                debug!(
                    "Copying persistent directory for: {} to: {}",
                    inner_path.to_string_lossy(),
                    new_path.to_string_lossy()
                );

                match fs_obj(&new_path).await {
                    FsObjectType::Directory => {
                        error!(
                            "Directory already exists for persistent directory: {}",
                            inner_path.to_string_lossy()
                        );
                        return Err(CopyPersistentError::DirectoryAlreadyExists(new_path));
                    }
                    FsObjectType::File => {
                        error!(
                            "File already exists for persistent directory: {}",
                            inner_path.to_string_lossy()
                        );
                        return Err(CopyPersistentError::FileAlreadyExists(new_path));
                    }
                    FsObjectType::None => {}
                }

                let new_path = new_path.clone();
                let copy_path = current_path.clone();
                match spawn_blocking(move || copy_dir(&copy_path, new_path)).await {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("Errors ocurred during directory copy: {:#?}", e);
                        return Err(CopyPersistentError::DirectoryCopyFailed(current_path));
                    }
                }

                info!(
                    "Copied persistent directory: {}",
                    inner_path.to_string_lossy()
                );
            }
            PersistentObject::File(inner_path) => {
                let current_path = store.build_path.join(inner_path.clone());
                let new_path = new_path.join(&inner_path);

                if fs_obj(&current_path).await != FsObjectType::File {
                    error!(
                        "No file for persistent object: {}",
                        inner_path.to_string_lossy()
                    );
                    continue;
                }

                debug!(
                    "Copying persistent file for: {} to: {}",
                    inner_path.to_string_lossy(),
                    new_path.to_string_lossy()
                );

                match fs_obj(&new_path).await {
                    FsObjectType::Directory => {
                        error!(
                            "Directory already exists for persistent directory: {}",
                            inner_path.to_string_lossy()
                        );
                        return Err(CopyPersistentError::DirectoryAlreadyExists(new_path));
                    }
                    FsObjectType::File => {
                        error!(
                            "File already exists for persistent directory: {}",
                            inner_path.to_string_lossy()
                        );
                        return Err(CopyPersistentError::FileAlreadyExists(new_path));
                    }
                    FsObjectType::None => {}
                }

                match fs::copy(&current_path, new_path).await {
                    Ok(_) => {}
                    Err(e) => return Err(CopyPersistentError::Filesystem(e)),
                }

                info!("Copied persistent file: {}", inner_path.to_string_lossy());
            }
        }
    }

    Ok(())
}
