use std::path::PathBuf;

use crate::fsobj::CreateAncestorError;

mod export;
mod import;
mod info;

pub use import::import;
pub use info::ExportInfo;

pub const EXPORT_INFO_FILENAME: &str = ".vkexport.json";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File \"{path}\" couldn't be located while walking: {err}")]
    WalkDirFile { path: PathBuf, err: String },
    #[error("Abrupt error while walking: {0}")]
    WalkDirAbrupt(String),
    #[error("Directory not found: {0}")]
    DirectoryMissing(PathBuf),
    #[error("Unable to find object to export (\"{0}\", outdated export info?)")]
    ExportableNotFound(PathBuf),
    #[error("Filesystem error: {0}")]
    Filesystem(std::io::Error),
    #[error("Tar archive assembly error: {0}")]
    TarBuilder(std::io::Error),
    #[error("Tar archive read error: {0}")]
    TarReader(std::io::Error),
    #[error("JSON error: {0}")]
    Json(serde_jsonc::Error),
    #[error("Not an export archive")]
    NotAnExportArchive,
    #[error("Not found in archive: {0}")]
    NotInArchive(PathBuf),
    #[error("Create path ancestors error: {0}")]
    CreateAncestor(CreateAncestorError),
}
