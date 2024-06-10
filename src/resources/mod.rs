use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod conf;
mod jdk;
pub mod style;

pub use jdk::{Jdk, JdkConfig};

const JDK_FILE: &str = include_str!("jdk.yml");

#[derive(Debug, thiserror::Error)]
pub enum ResourceLoadError {
    #[error("Failed to parse YAML: {0}")]
    YamlParse(serde_yaml::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
pub enum ArchiveFormat {
    #[serde(rename = "tar.gz")]
    TarGz,
    #[serde(rename = "zip")]
    Zip,
}
