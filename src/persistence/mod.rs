use serde::{Deserialize, Serialize};
use std::path;

pub mod copy;

// pub use info::{PersistInfo, PersistentInfoError, PersistInfoFile};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PersistentObject {
    #[serde(rename = "directory")]
    Directory(path::PathBuf),
    #[serde(rename = "file")]
    File(path::PathBuf),
}
