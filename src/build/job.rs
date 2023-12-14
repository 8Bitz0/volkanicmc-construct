use base64::Engine;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::path;
use tokio::fs;
use tracing::error;

use crate::resources::{self, Jdk, JdkConfig};
use crate::template::{self, vkinclude};
use crate::vkstore;

use super::buildinfo;
use super::misc;
use super::prepare_jdk;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("No JDK found for your system (version: {0})")]
    JdkNotFound(String),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
    #[error("No file name found in path: {0}")]
    NoFileNameInPath(path::PathBuf),
    #[error("Creating path ancestor directories failed: {0}")]
    CreateFilesystemAncestors(misc::CreateAncestorError),
    #[error("Base64 error: {0}")]
    Base64(base64::DecodeError),
    #[error("Download error: {0}")]
    Download(misc::DownloadError),
    #[error("Prepare JDK error: {0}")]
    PrepareJdk(prepare_jdk::PrepareJdkError),
    #[error("Build info error: {0}")]
    BuildInfo(buildinfo::BuildInfoError),
    #[error("Not available in Volkanic include folder: {0}")]
    NotAvailableInIncludeFolder(String),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum JobAction {
    /// Create a directory
    #[serde(rename = "create-dir")]
    CreateDir { path: path::PathBuf },
    /// Create a file with contents from Base64
    #[serde(rename = "create-file-base64")]
    WriteFileBase64 {
        path: path::PathBuf,
        contents: String,
    },
    /// Download a file
    #[serde(rename = "download-file")]
    WriteFileRemote {
        path: path::PathBuf,
        url: String,
        sha512: Option<String>,
    },
    /// Copy a file
    #[serde(rename = "from-include")]
    CopyFromInclude {
        id: String,
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    /// Setup JDK
    #[serde(rename = "prepare-jdk")]
    PrepareJdk { jdk: Jdk },
}

impl JobAction {
    pub async fn execute(&self, store: &vkstore::VolkanicStore) -> Result<(), JobError> {
        match self {
            JobAction::CreateDir {
                path: template_path,
            } => {
                fs::create_dir_all(store.build_path.join(template_path))
                    .await
                    .map_err(JobError::Filesystem)?;
            }
            JobAction::WriteFileBase64 {
                path: template_path,
                contents,
            } => {
                misc::create_ancestors(template_path.clone())
                    .await
                    .map_err(JobError::CreateFilesystemAncestors)?;

                let base64_config = base64::engine::GeneralPurposeConfig::new();
                let base64_engine =
                    base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64_config);

                let contents = base64_engine.decode(contents).map_err(|err| {
                    error!("Failed to decode base64: {}", err);
                    JobError::Base64(err)
                })?;

                fs::write(store.build_path.join(template_path), contents)
                    .await
                    .map_err(JobError::Filesystem)?;
            }
            JobAction::WriteFileRemote {
                path: template_path,
                url,
                sha512,
            } => {
                let abs_path = store.build_path.join(template_path);

                misc::create_ancestors(abs_path.clone())
                    .await
                    .map_err(JobError::CreateFilesystemAncestors)?;

                let name = if let Some(name) = template_path.file_name() {
                    name.to_string_lossy().to_string()
                } else {
                    return Err(JobError::NoFileNameInPath(abs_path));
                };

                let p = misc::download_indicatif(
                    store.clone(),
                    url,
                    match sha512 {
                        Some(sha512) => misc::Verification::Sha512(sha512.to_string()),
                        None => misc::Verification::None,
                    },
                    name,
                )
                .map_err(JobError::Download)
                .await?;

                fs::copy(p, abs_path).await.map_err(JobError::Filesystem)?;
            }
            JobAction::CopyFromInclude { id, template_path } => {
                let abs_path = store.build_path.join(template_path);

                misc::create_ancestors(abs_path.clone())
                    .await
                    .map_err(JobError::CreateFilesystemAncestors)?;

                let include = vkinclude::VolkanicInclude::new().await;

                let p = match include.get(id) {
                    Some(p) => p,
                    None => return Err(JobError::NotAvailableInIncludeFolder(id.to_string())),
                };

                fs::copy(p, store.build_path.join(template_path))
                    .await
                    .map_err(JobError::Filesystem)?;
            }
            JobAction::PrepareJdk { jdk } => {
                prepare_jdk::prepare_jdk(store.clone(), jdk.clone())
                    .await
                    .map_err(JobError::PrepareJdk)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Job {
    title: String,
    action: JobAction,
}

pub async fn create_jobs(
    template: &crate::template::Template,
    jdk_config: JdkConfig,
) -> Result<Vec<Job>, JobError> {
    let mut jobs = vec![];

    // Setup JDK
    match &template.runtime {
        template::resource::ServerRuntimeResource::Jdk { version } => {
            jobs.push(Job {
                title: "Prepare JDK".into(),
                action: JobAction::PrepareJdk {
                    jdk: match jdk_config.find(&version, None).await {
                        Some(jdk) => jdk,
                        None => {
                            error!("No JDK found for your system (version: {})", version);
                            return Err(JobError::JdkNotFound(version.to_string()));
                        }
                    },
                },
            });
        }
    }

    // Setup server software
    match &template.server {
        template::resource::ServerExecResource::Java { url, sha512 } => {
            jobs.push(Job {
                title: "Download server software".into(),
                action: JobAction::WriteFileRemote {
                    path: resources::conf::SERVER_SOFTWARE_FILE.into(),
                    url: url.clone(),
                    sha512: Some(sha512.clone()),
                },
            });
        }
    }

    // Setup additional resources
    for resource in &template.resources {
        match resource {
            template::resource::GenericResource::Remote {
                url,
                sha512,
                template_path: path,
            } => {
                jobs.push(Job {
                    title: "Download additional resource".into(),
                    action: JobAction::WriteFileRemote {
                        path: path.clone(),
                        url: url.clone(),
                        sha512: sha512.clone(),
                    },
                });
            }
            template::resource::GenericResource::Base64 {
                base64: base,
                template_path,
            } => {
                jobs.push(Job {
                    title: "Write file from Base64".into(),
                    action: JobAction::WriteFileBase64 {
                        path: template_path.clone(),
                        contents: base.clone(),
                    },
                });
            }
            template::resource::GenericResource::Include {
                include_id,
                template_path,
            } => {
                jobs.push(Job {
                    title: "Copy additional resource".into(),
                    action: JobAction::CopyFromInclude {
                        id: include_id.to_string(),
                        template_path: template_path.clone(),
                    },
                });
            }
            template::resource::GenericResource::Modrinth { identity: _ } => todo!(),
        }
    }

    Ok(jobs)
}

pub async fn execute_jobs(
    store: vkstore::VolkanicStore,
    build_info: &mut buildinfo::BuildInfo,
) -> Result<(), JobError> {
    build_info.job_progress = 0;

    for job in &build_info.jobs {
        job.action.execute(&store).await?;

        build_info.job_progress += 1;

        build_info.update().await.map_err(JobError::BuildInfo)?;
    }

    Ok(())
}
