use base64::Engine;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::path;
use tokio::fs;
use tracing::error;

use crate::resources::{Jdk, JdkConfig};
use crate::template;
use crate::vkstore;

use super::lock;
use super::misc;
use super::prepare_jdk;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("No JDK found for your system (version: {0})")]
    JdkNotFound(String),
    #[error("Filesystem error: {0}")]
    FilesystemError(tokio::io::Error),
    #[error("Base64 error: {0}")]
    Base64Error(base64::DecodeError),
    #[error("Download error: {0}")]
    DownloadError(misc::DownloadError),
    #[error("Prepare JDK error: {0}")]
    PrepareJdkError(prepare_jdk::PrepareJdkError),
    #[error("Lock error: {0}")]
    LockError(lock::LockError),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum JobAction {
    /// Create a directory
    #[serde(rename = "create-dir")]
    CreateDir { path: path::PathBuf },
    /// Create a file with contents from Base64
    #[serde(rename = "create-file-base64")]
    WriteFileBase64 { path: path::PathBuf, contents: String },
    /// Download a file
    #[serde(rename = "download-file")]
    WriteFileRemote { path: path::PathBuf, url: String, sha512: Option<String> },
    /// Copy a file
    #[serde(rename = "copy-file")]
    CopyFile { orig_path: path::PathBuf, dest_path: path::PathBuf },
    /// Setup JDK
    #[serde(rename = "prepare-jdk")]
    PrepareJdk { jdk: Jdk },
}

impl JobAction {
    pub async fn execute(&self, store: &vkstore::VolkanicStore) -> Result<(), JobError> {
        match self {
            JobAction::CreateDir { path } => {
                fs::create_dir_all(store.build_path.join(path)).await.map_err(JobError::FilesystemError)?;
            }
            JobAction::WriteFileBase64 { path, contents } => {
                let base64_config = base64::engine::GeneralPurposeConfig::new();
                let base64_engine = base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64_config);

                let contents = base64_engine.decode(contents).map_err(|err| {
                    error!("Failed to decode base64: {}", err);
                    JobError::Base64Error(err)
                })?;

                fs::write(store.build_path.join(path), contents).await.map_err(JobError::FilesystemError)?;
            }
            JobAction::WriteFileRemote { path, url, sha512 } => {
                let p = misc::download(store.clone(), url, match sha512 {
                    Some(sha512) => misc::Verification::Sha512(sha512.to_string()),
                    None => misc::Verification::None,
                }, path.to_path_buf()).map_err(JobError::DownloadError).await?;

                fs::copy(p, store.build_path.join(path)).await.map_err(JobError::FilesystemError)?;
            }
            JobAction::CopyFile { orig_path, dest_path } => {
                fs::copy(orig_path, store.build_path.join(dest_path)).await.map_err(JobError::FilesystemError)?;
            }
            JobAction::PrepareJdk { jdk } => {
                prepare_jdk::prepare_jdk(store.clone(), jdk.clone()).await.map_err(JobError::PrepareJdkError)?;
            },
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Job {
    title: String,
    action: JobAction,
}

pub async fn create_jobs(template: &crate::template::Template, jdk_config: JdkConfig) -> Result<Vec<Job>, JobError> {
    let mut jobs = vec![];

    // Setup JDK
    match &template.runtime {
        template::resource::ServerRuntimeResource::Jdk { version } => {
            jobs.push(Job {
                title: "Prepare JDK".into(),
                action: JobAction::PrepareJdk { jdk: match jdk_config.find(&version, None).await {
                    Some(jdk) => jdk,
                    None => {
                        error!("No JDK found for your system (version: {})", version);
                        return Err(JobError::JdkNotFound(version.to_string()));
                    }
                } },
            });
        }
    }

    // Setup server software
    match &template.server {
        template::resource::ServerExecResource::Java { url, sha512 } => {
            jobs.push(Job {
                title: "Download server software".into(),
                action: JobAction::WriteFileRemote { path: path::PathBuf::from("server.jar"), url: url.clone(), sha512: Some(sha512.clone()) },
            });
        }
    }

    // Setup additional resources
    for resource in &template.resources {
        match resource {
            template::resource::GenericResource::Remote { url, sha512, path } => {
                jobs.push(Job {
                    title: "Download additional resource".into(),
                    action: JobAction::WriteFileRemote { path: path::PathBuf::from(path.clone()), url: url.clone(), sha512: sha512.clone() },
                });
            }
            template::resource::GenericResource::FsCopy { path, template_path } => {
                jobs.push(Job {
                    title: "Copy additional resource".into(),
                    action: JobAction::CopyFile { orig_path: path.clone(), dest_path: template_path.clone() },
                });
            }
            template::resource::GenericResource::Modrinth { identity: _ } => todo!(),
        }
    }

    Ok(jobs)
}

pub async fn execute_jobs(store: vkstore::VolkanicStore, lock: &mut lock::Lock) -> Result<(), JobError> {
    let mut to_skip = lock.job_progress;

    for job in &lock.jobs {
        if to_skip > 0 {
            to_skip -= 1;
            continue;
        }

        job.action.execute(&store).await?;

        lock.job_progress += 1;
        
        lock.update().await.map_err(JobError::LockError)?;
    }

    Ok(())
}
