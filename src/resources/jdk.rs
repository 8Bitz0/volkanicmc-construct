use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use sysinfo::{System, SystemExt};

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
    pub platforms: BTreeMap<String, Jdk>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkPlatforms {
    #[serde(flatten)]
    pub architectures: BTreeMap<String, JdkArchitectures>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkVersions {
    #[serde(flatten)]
    pub versions: BTreeMap<String, JdkPlatforms>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JdkConfig {
    versions: Vec<JdkVersions>
}

impl JdkConfig {
    pub async fn parse_list() -> Result<JdkConfig, ResourceLoadError> {
        let resource_name = super::JDK_FILE;
        
        Ok(JdkConfig {
            versions: serde_yaml::from_str::<Vec<JdkVersions>>(resource_name).map_err(ResourceLoadError::YamlParseError)?
        })
    }
    pub async fn find(&self, version: impl std::fmt::Display, override_sys: Option<(String, String)>) -> Option<Jdk> {
        let os = if let Some((os, _)) = override_sys.clone() {
            os
        } else {
            match std::env::consts::OS {
                "android" => "android",
                "freebsd" => "freebsd",
                "ios" => "ios",
                "macos" => "mac",
                "netbsd" => "netbsd",
                "openbsd" => "openbsd",
                "dragonfly" => "dragonfly",
                "solaris" => "solaris",
                "windows" => "windows",
                "linux" => {
                    let mut sys = System::new();
                    sys.refresh_system();
    
                    match sys.distribution_id().as_str() {
                        "alpine" => "alpine",
                        _ => "linux",
                    }
                },
                _ => return None,
            }.to_string()
        };
    
        let arch = if let Some((_, arch)) = override_sys.clone() {
            arch
        } else {
            match std::env::consts::ARCH {
                "x86" => "x86",
                "x86_64" => "amd64",
                "arm" => "arm",
                "aarch64" => "arm64",
                "m68k" => "m68k",
                "csky" => "csky",
                "mips" => "mips",
                "mips64" => "mips64",
                "powerpc" => "ppc",
                "powerpc64" => "ppc64",
                "riscv64" => "riscv64",
                "s390x" => "s390x",
                "sparc64" => "sparc64",
                _ => return None,
            }.to_string()
        };
    
        self
            .versions
            .iter()
            .flat_map(|jdk_versions| jdk_versions.versions.get(&version.to_string()))
            .flat_map(|jdk_platforms| jdk_platforms.architectures.get(&os))
            .flat_map(|jdk_architectures| jdk_architectures.platforms.get(&arch))
            .next().cloned()
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
        let jdk = jdk_list.find("8".to_string(), Some(("linux".to_string(), "amd64".to_string()))).await;
        println!("{:#?}", jdk);
        assert!(jdk.is_some());
    }
}
