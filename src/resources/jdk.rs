use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::hostinfo;

use super::{ArchiveFormat, ResourceLoadError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jdk {
    pub url: String,
    pub sha256: String,
    #[serde(rename = "home-dir")]
    pub home_path: String,
    pub format: ArchiveFormat,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkArchitectures {
    #[serde(flatten)]
    pub platforms: BTreeMap<hostinfo::Arch, Jdk>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkPlatforms {
    #[serde(flatten)]
    pub architectures: BTreeMap<hostinfo::Os, JdkArchitectures>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkVersions {
    #[serde(flatten)]
    pub versions: BTreeMap<String, JdkPlatforms>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkConfig {
    versions: Vec<JdkVersions>,
}

impl JdkConfig {
    pub async fn parse_list() -> Result<JdkConfig, ResourceLoadError> {
        let resource_name = super::JDK_FILE;

        Ok(JdkConfig {
            versions: serde_yaml::from_str::<Vec<JdkVersions>>(resource_name)
                .map_err(ResourceLoadError::YamlParse)?,
        })
    }
    pub async fn find(
        &self,
        version: impl std::fmt::Display,
        override_sys: Option<(hostinfo::Os, hostinfo::Arch)>,
    ) -> Option<Jdk> {
        let os = if let Some((os, _)) = override_sys.clone() {
            os
        } else {
            hostinfo::Os::get().await?
        };

        let arch = if let Some((_, arch)) = override_sys.clone() {
            arch
        } else {
            hostinfo::Arch::get().await?
        };

        self.versions
            .iter()
            .flat_map(|jdk_versions| jdk_versions.versions.get(&version.to_string()))
            .flat_map(|jdk_platforms| jdk_platforms.architectures.get(&os))
            .flat_map(|jdk_architectures| jdk_architectures.platforms.get(&arch))
            .next()
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get() {
        let jdk_list = JdkConfig::parse_list().await.unwrap();
        println!("{:#?}", jdk_list);
        assert!(!jdk_list.versions.is_empty());
    }

    #[tokio::test]
    async fn test_find() {
        let jdk_list = JdkConfig::parse_list().await.unwrap();
        let jdk = jdk_list
            .find(
                "8".to_string(),
                Some((hostinfo::Os::Linux, hostinfo::Arch::Amd64)),
            )
            .await;
        println!("{:#?}", jdk);
        assert!(jdk.is_some());
    }
}
