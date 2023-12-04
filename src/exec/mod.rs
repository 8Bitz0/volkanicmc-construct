use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::hostinfo;

mod run;

pub use run::run;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildExecInfo {
    /// Target system architecture
    pub arch: hostinfo::Arch,
    /// Target operating system
    pub os: hostinfo::Os,
    /// Path to the runtime executable relative to the runtime directory.
    #[serde(rename = "runtime-exec-path")]
    pub runtime_exec_path: PathBuf,
    /// Path to the server software JAR relative to the build directory.
    #[serde(rename = "server-jar-path")]
    pub server_jar_path: PathBuf,
    /// Arguments for the runtime executable
    #[serde(rename = "runtime-args")]
    pub runtime_args: Vec<String>,
    /// Arguments for the server executable
    #[serde(rename = "server-args")]
    pub server_args: Vec<String>,
}
