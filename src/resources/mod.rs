use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod conf;
mod jdk;
pub mod style;

pub use jdk::{HomePathType, Jdk, JdkLookup};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Foojay Disco error: {0}")]
    FoojayDisco(foojay_disco::Error),
    #[error("Failed to fetch system architecture")]
    UnknownArchitecture,
    #[error("Failed to fetch operating system")]
    UnknownOperatingSystem,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
pub enum ArchiveFormat {
    #[serde(rename = "tar.gz")]
    TarGz,
    #[serde(rename = "zip")]
    Zip,
}
