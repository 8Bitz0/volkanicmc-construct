use foojay_disco::{self, PackageQueryOptions};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::hostinfo::{self, Os};

use super::{ArchiveFormat, Error};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HomePathType {
    FirstSubDir,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jdk {
    pub url: String,
    pub sha256: Option<String>,
    #[serde(rename = "home-dir")]
    pub home_path: HomePathType,
    pub format: ArchiveFormat,
}

pub struct JdkLookup {}

impl JdkLookup {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn find(
        &self,
        version: impl std::fmt::Display,
        override_sys: Option<(hostinfo::Os, hostinfo::Arch)>,
    ) -> Result<Option<Jdk>, Error> {
        // Get current operating system
        let os = if let Some((os, _)) = override_sys.clone() {
            os
        } else {
            match hostinfo::Os::get().await {
                Some(o) => o,
                None => return Err(Error::UnknownOperatingSystem),
            }
        };

        // Get current architecture
        let arch = if let Some((_, arch)) = override_sys.clone() {
            arch
        } else {
            match hostinfo::Arch::get().await {
                Some(o) => o,
                None => return Err(Error::UnknownArchitecture),
            }
        };

        let archive_type = if os == Os::Windows { "zip".to_string() } else { "tar.gz".to_string() };

        info!("Fetching JDK packages...");

        // Pull packages from Disco
        let packages = tokio::task::spawn_blocking(move || {
            foojay_disco::pull_packages(option_env!("FOOJAY_DISCO_URL"), Some(PackageQueryOptions {
                architecture: Some(arch.to_string()),
                operating_system: Some(os.to_string()),
                archive_type: Some(archive_type),
                directly_downloadable: Some(true),
                bitness: None,
                distribution: None,
                javafx_bundled: None,
                latest: None,
                libc_type: None,
                package_type: None,
                release_status: None,
                term_of_support: None,
                version: None,
            }))
        }).await.unwrap();

        let packages = match packages {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to fetch JDK packages via Foojay Disco: {e}");
                return Err(Error::FoojayDisco(e));
            }
        };

        // Filter out packages which don't match the correct version
        for p in packages.result {
            if p.major_version.to_string() != version.to_string() {
                continue;
            }

            let download_url = match p.links.get("pkg_download_redirect") {
                Some(u) => u,
                None => continue,
            };

            // Construct the JDK object with information found from Disco
            return Ok(Some(Jdk {
                format: match p.archive_type.as_str() {
                    "tar.gz" => ArchiveFormat::TarGz,
                    "zip" => ArchiveFormat::Zip,
                    _ => continue,
                },
                home_path: HomePathType::FirstSubDir,
                sha256: None,
                url: download_url.to_string(),
            }));
        };

        // No compatible package was found
        Ok(None)
    }
}
