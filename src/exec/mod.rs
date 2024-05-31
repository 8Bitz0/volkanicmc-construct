use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::hostinfo;

pub mod script;

mod run;

pub use run::run;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildExecInfo {
    /// Target system architecture
    pub arch: hostinfo::Arch,
    /// Target operating system
    pub os: hostinfo::Os,
    /// Path to the runtime executable relative to the runtime directory.
    #[serde(rename = "exec-path")]
    pub exec_path: PathBuf,
    /// Arguments for the runtime executable
    #[serde(rename = "runtime-args")]
    pub args: Vec<String>,
}
