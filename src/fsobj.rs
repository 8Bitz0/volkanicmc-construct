use std::{io, path};
use tokio::fs;
use tracing::{debug, error};

#[derive(Debug, PartialEq)]
pub enum FsObjectType {
    None,
    File,
    Directory,
}

impl std::fmt::Display for FsObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub async fn fs_obj<P: AsRef<path::Path>>(path: P) -> FsObjectType {
    if path.as_ref().is_file() {
        FsObjectType::File
    } else if path.as_ref().is_dir() {
        FsObjectType::Directory
    } else {
        FsObjectType::None
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CreateAncestorError {
    #[error("Filesystem error: {0}")]
    FilesystemError(io::Error),
    #[error("Found wrong filesystem object at: \"{0}\" (expected: {1}, found: {2})")]
    WrongFsObject(path::PathBuf, FsObjectType, FsObjectType),
    #[error("No parent directory for path: {0}")]
    NoParentDir(path::PathBuf),
}

pub async fn create_ancestors<P: AsRef<path::Path>>(path: P) -> Result<(), CreateAncestorError> {
    debug!(
        "Creating ancestors for \"{}\"",
        path.as_ref().to_string_lossy()
    );
    if let Some(parent) = path.as_ref().parent() {
        debug!("Direct parent path: \"{}\"", parent.to_string_lossy());
        match fs_obj(path.as_ref()).await {
            FsObjectType::Directory => {
                debug!(
                    "Ancestors already exist for \"{}\"",
                    path.as_ref().to_string_lossy()
                );
                Ok(())
            }
            FsObjectType::None => {
                fs::create_dir_all(parent)
                    .await
                    .map_err(CreateAncestorError::FilesystemError)?;

                Ok(())
            }
            _ => {
                error!(
                    "Wrong filesystem object at: \"{}\"",
                    path.as_ref().to_string_lossy()
                );
                Err(CreateAncestorError::WrongFsObject(
                    path.as_ref().to_path_buf(),
                    FsObjectType::Directory,
                    fs_obj(path).await,
                ))
            }
        }
    } else {
        Err(CreateAncestorError::NoParentDir(
            path.as_ref().to_path_buf(),
        ))
    }
}
