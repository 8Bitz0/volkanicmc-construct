use serde::{Deserialize, Serialize};

pub mod conf;
mod jdk;

pub use jdk::{Jdk, JdkArchitectures, JdkConfig, JdkPlatforms, JdkVersions};

const JDK_FILE: &str = include_str!("jdk.yml");

#[derive(Debug, thiserror::Error)]
pub enum ResourceLoadError {
    #[error("Failed to parse YAML: {0}")]
    YamlParse(serde_yaml::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ArchiveFormat {
    #[serde(rename = "tar.gz")]
    TarGz,
    #[serde(rename = "zip")]
    Zip,
}
