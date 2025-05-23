use foojay_disco::{self, PackageQueryOptions};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::hostinfo::{self, Os};

use super::{ArchiveFormat, Error};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HomePathType {
    Auto,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jdk {
    pub url: String,
    pub file_name: Option<String>,
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
        force_jdk_distribution: Option<String>,
        preferred_distributions: Option<Vec<String>>,
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

        let version_str = version.to_string();

        // Pull packages from Disco
        let packages = tokio::task::spawn_blocking(move || {
            foojay_disco::pull_packages(option_env!("FOOJAY_DISCO_URL"), Some(PackageQueryOptions {
                architecture: Some(arch.to_string()),
                operating_system: Some(os.to_string()),
                archive_type: Some(archive_type),
                directly_downloadable: Some(true),
                bitness: None,
                distribution: force_jdk_distribution,
                javafx_bundled: None,
                // Make sure on the latest available packages are pulled
                latest: Some("available".to_string()),
                libc_type: None,
                package_type: None,
                release_status: None,
                term_of_support: None,
                version: Some(version_str),
            }))
        }).await.unwrap();

        let packages = match packages {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to fetch JDK packages via Foojay Disco: {e}");
                return Err(Error::FoojayDisco(e));
            }
        };

        // Sort packages by preferred distributions if specified
        let mut packages = packages.result;
        if let Some(preferred) = preferred_distributions {
            packages.sort_by_key(|p| {
                preferred.iter()
                    .position(|d| d == &p.distribution)
                    .unwrap_or(usize::MAX)
            });
        }

        // Filter out packages which don't match the correct version
        for p in packages {
            if p.major_version.to_string() != version.to_string() {
                continue;
            }

            info!("Fetching package info...");

            // Some information like the file checksum is only available via another request
            let package_info = tokio::task::spawn_blocking(move || {
                foojay_disco::pull_package_info(option_env!("FOOJAY_DISCO_URL"), p.id)
            }).await.unwrap();

            let package_info = match package_info {
                Ok(o) => o.result[0].clone(),
                Err(e) => {
                    error!("Failed to fetch package information by ID via Foojay Disco: {e}");
                    return Err(Error::FoojayDisco(e));
                }
            };

            // Each field can be an empty string, so we ignore those and continue on
            if package_info.direct_download_uri.is_empty() {
                continue;
            }

            // Construct doesn't support any verification types other than SHA256
            let mut verification = if package_info.checksum_type == "sha256" {
                Some(package_info.checksum.clone())
            } else {
                None
            };

            // In case the JDK still doesn't have a valid checksum
            if package_info.checksum.len() != 64 {
                debug!("Empty checksum from JDK");
                verification = None
            }

            // Construct the JDK object with information found from Disco
            return Ok(Some(Jdk {
                file_name: Some(p.filename),
                format: match p.archive_type.as_str() {
                    "tar.gz" => ArchiveFormat::TarGz,
                    "zip" => ArchiveFormat::Zip,
                    _ => continue,
                },
                home_path: HomePathType::Auto,
                sha256: verification,
                url: package_info.direct_download_uri.to_string(),
            }));
        };

        // No compatible package was found
        Ok(None)
    }
}
