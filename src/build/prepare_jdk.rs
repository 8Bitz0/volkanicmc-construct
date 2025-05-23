use std::path::{self, Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info};

use crate::resources::{self, HomePathType, Jdk};
use crate::vkstore;

use super::misc::{
    download_progress, extract, get_remote_filename, DownloadError, ExtractionError, Verification,
};

const DEFAULT_JDK_PKG_NAME: &str = "unknown-jdk-runtime";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Download error: {0}")]
    Download(DownloadError),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
    #[error("Extraction error: {0}")]
    Extraction(ExtractionError),
    #[error("Invalid JDK home directory: {0}")]
    InvalidJdkHome(path::PathBuf),
    #[error("Failed to detect JDK home directory")]
    JdkHomeUndetected,
    #[error("Directory failed to be copied: {0}")]
    DirectoryCopyFailed(path::PathBuf),
    #[error("No name for URL: {0}")]
    NoNameForUrl(String),
}

/// Checks if a given path points to a valid JDK home directory.
///
/// A directory is considered a JDK home if it exists, is a directory,
/// and contains `bin/java` or `bin/`
async fn is_valid_jdk_home(path_to_check: &Path) -> bool {
    // Ensure the path itself is a directory
    match fs::metadata(path_to_check).await {
        Ok(meta) if meta.is_dir() => {
            // Path is a directory, now check for Java executables
        }
        _ => return false, // Not a directory or error accessing metadata
    }

    let bin_dir = path_to_check.join("bin");

    if !bin_dir.is_dir() | bin_dir.is_symlink() {
        return false;
    }

    let exec_names = ["java", "java.exe"];

    for e in exec_names {
        let java_path = bin_dir.join(e);

        // Check if java executable exists and is a file
        match fs::metadata(&java_path).await {
            Ok(meta) => {
                if meta.is_file() {
                    return true;
                }
            },
            Err(e) => {
                debug!("Did not find Java executable: {} (error: {})", java_path.to_string_lossy(), e);
            },
        };
    }

    false
}

/// Reads a directory and tries to find a JDK home within it or its direct subdirectories.
///
/// It first checks if the `base_dir` itself is a JDK home.
/// If not, it then checks each direct subdirectory of `base_dir`.
/// Returns `Ok(Some(PathBuf))` if a JDK home is found, `Ok(None)` if not found,
/// or `Err(io::Error)` if there's an issue reading the directory.
async fn find_jdk_home<P: AsRef<Path>>(base_dir: P) -> Result<Option<PathBuf>, std::io::Error> {
    let base_path = base_dir.as_ref().to_path_buf();

    // 1. Check if the base_dir itself is a JDK home
    if is_valid_jdk_home(&base_path).await {
        return Ok(Some(base_path));
    }

    // 2. Check all subdirectories recursively using a stack-based approach
    let mut dirs_to_check = vec![base_path.clone()];

    while let Some(current_dir) = dirs_to_check.pop() {
        let mut entries = match fs::read_dir(&current_dir).await {
            Ok(entries) => entries,
            Err(e) => {
                debug!("Error reading directory {:?}: {}", current_dir, e);
                continue;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            
            if let Ok(metadata) = fs::metadata(&entry_path).await {
                // Check if the entry is a directory
                if metadata.is_dir() {
                    // Check if the directory is a valid Java home
                    if is_valid_jdk_home(&entry_path).await {
                        debug!("{} is a valid JDK home", entry_path.to_string_lossy());
                        // Remove parts of the path before the package root
                        match entry_path.strip_prefix(&base_dir) {
                            Ok(relative) => return Ok(Some(relative.to_path_buf())),
                            Err(e) => {
                                error!(
                                    "Failed to find relative path for: {}, error: {}", 
                                    entry_path.to_string_lossy(), 
                                    e
                                );
                            }
                        }
                    } else {
                        // Add this directory to check its contents later
                        dirs_to_check.push(entry_path);
                    }
                }
            }
        }
    }

    Ok(None) // No JDK home found
}

pub async fn prepare_jdk(
    store: vkstore::VolkanicStore,
    jdk: Jdk,
    no_verify: bool,
) -> Result<(), Error> {
    let mut jdk_name = jdk.file_name.clone().unwrap_or(match get_remote_filename(&jdk.url).await {
        Some(s) => s,
        None => return Err(Error::NoNameForUrl(jdk.url.clone())),
    });

    if jdk_name.is_empty() {
        jdk_name = DEFAULT_JDK_PKG_NAME.to_string();
    }

    let jdk_path = download_progress(
        store.clone(),
        &jdk.url,
        // Ignore verification parameters if the no verify flag is enabled
        if no_verify {
            Verification::None
        } else if let Some(sha256) = jdk.sha256 {
            Verification::Sha256(sha256)
        } else {
            Verification::None
        },
        jdk_name,
        None::<String>,
    )
    .await
    .map_err(Error::Download)?;

    let ex_path = extract(store.clone(), jdk_path, jdk.format)
        .await
        .map_err(Error::Extraction)?;

    let home_path = match jdk.home_path {
        HomePathType::Custom(p) => p,
        HomePathType::Auto => {
            match find_jdk_home(&ex_path).await {
                Ok(p) => {
                    if let Some(p) = p {
                        info!("Found JDK home: {}", p.to_string_lossy());
                        p.to_string_lossy().to_string()
                    } else {
                        return Err(Error::JdkHomeUndetected);
                    }
                }
                Err(e) => {
                    error!("Failed to find JDK home: {}", e);
                    return Err(Error::Filesystem(e));
                }
            }
        }
    };

    if !ex_path
        .join(&home_path)
        .join(resources::conf::JDK_BIN_FILE)
        .is_file()
    {
        return Err(Error::InvalidJdkHome(ex_path.join(home_path)));
    }

    if store.runtime_path.is_dir() {
        info!("Removing existing runtime directory");
        fs::remove_dir_all(&store.runtime_path)
            .await
            .map_err(Error::Filesystem)?;
    }

    match copy_dir::copy_dir(ex_path.join(&home_path), &store.runtime_path) {
        Ok(_) => {
            info!("Copied JDK to runtime directory");
        }
        Err(e) => {
            debug!("Errors ocurred during JDK copy: {:#?}", e);
            return Err(Error::DirectoryCopyFailed(
                ex_path.join(home_path),
            ));
        }
    }

    Ok(())
}
