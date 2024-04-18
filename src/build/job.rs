use base64::Engine;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, error, info};

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
    #[error("Inner archive path doesn't exist: {0}")]
    InnerArchivePathNotFound(path::PathBuf),
    #[error("Extraction error: {0}")]
    ExtractionError(misc::ExtractionError),
    #[error("No file name found in path: {0}")]
    NoFileNameInPath(path::PathBuf),
    #[error("Creating path ancestor directories failed: {0}")]
    CreateFilesystemAncestors(misc::CreateAncestorError),
    #[error("Directory copy failed: {0}")]
    DirectoryCopyFailed(path::PathBuf),
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
    #[error("Archives cannot have variables (resource path: {0})")]
    ArchivesCannotHaveVariables(path::PathBuf),
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
        /// If format is not defined, the file is only copied and not decompressed.
        archive: Option<template::resource::ArchiveInfo>,
        url: String,
        sha512: Option<String>,
        #[serde(rename = "user-agent")]
        user_agent: Option<String>,
        #[serde(rename = "override-name")]
        override_name: Option<String>,
    },
    /// Copy a file
    #[serde(rename = "from-include")]
    CopyFromInclude {
        id: String,
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    ProcessVariables {
        path: path::PathBuf,
        format: template::var::VarFormat,
        variables: template::var::VarMap,
    },
    /// Setup JDK
    #[serde(rename = "prepare-jdk")]
    PrepareJdk { jdk: Jdk, no_verify: bool },
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
                let abs_path = store.build_path.join(template_path);

                misc::create_ancestors(abs_path.clone())
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
                archive,
                url,
                sha512,
                user_agent,
                override_name,
            } => {
                let abs_path = store.build_path.join(template_path);

                misc::create_ancestors(abs_path.clone())
                    .await
                    .map_err(JobError::CreateFilesystemAncestors)?;

                let name = {
                    match override_name {
                        Some(name) => name.clone(),
                        None => {
                            if let Some(name) = misc::get_remote_filename(url).await {
                                name
                            } else {
                                return Err(JobError::NoFileNameInPath(abs_path));
                            }
                        }
                    }
                };

                let p = misc::download_progress(
                    store.clone(),
                    url,
                    match sha512 {
                        Some(sha512) => misc::Verification::Sha512(sha512.to_string()),
                        None => misc::Verification::None,
                    },
                    name.to_string(),
                    user_agent.clone(),
                )
                .map_err(JobError::Download)
                .await?;

                match archive {
                    Some(t) => {
                        let archive_path =
                            misc::extract(store.clone(), p, t.archive_format.clone())
                                .await
                                .map_err(JobError::ExtractionError)?;
                        let a_path_inner = archive_path.join(t.inner_path.clone());

                        match misc::fs_obj(a_path_inner.clone()).await {
                            misc::FsObjectType::Directory => {
                                match copy_dir::copy_dir(&a_path_inner, &abs_path) {
                                    Ok(_) => {
                                        info!(
                                            "Copied resource directory \"{}\" to \"{}\"",
                                            a_path_inner.to_string_lossy(),
                                            abs_path.to_string_lossy()
                                        );
                                    }
                                    Err(e) => {
                                        debug!("Errors ocurred during JDK copy: {:#?}", e);
                                        return Err(JobError::DirectoryCopyFailed(a_path_inner));
                                    }
                                }

                                for p in &t.post_remove {
                                    let abs_rm_path = abs_path.join(p);

                                    match misc::fs_obj(abs_rm_path.clone()).await {
                                        misc::FsObjectType::Directory => {
                                            info!(
                                                "Remove post-removal directory: \"{}\"",
                                                p.to_string_lossy()
                                            );
                                            fs::remove_dir_all(abs_rm_path)
                                                .await
                                                .map_err(JobError::Filesystem)?;
                                        }
                                        misc::FsObjectType::File => fs::remove_file(abs_rm_path)
                                            .await
                                            .map_err(JobError::Filesystem)?,
                                        misc::FsObjectType::None => {
                                            error!(
                                                "Post-removal inner-archive path not found: {}",
                                                p.to_string_lossy()
                                            );
                                        }
                                    }
                                }
                            }
                            misc::FsObjectType::File => {
                                fs::copy(&a_path_inner, &abs_path)
                                    .await
                                    .map_err(JobError::Filesystem)?;
                            }
                            misc::FsObjectType::None => {
                                return Err(JobError::InnerArchivePathNotFound(a_path_inner))
                            }
                        }
                    }
                    None => {
                        fs::copy(p, abs_path).await.map_err(JobError::Filesystem)?;
                    }
                }
            }
            JobAction::CopyFromInclude { id, template_path } => {
                let abs_path = store.build_path.join(template_path);

                misc::create_ancestors(abs_path.clone())
                    .await
                    .map_err(JobError::CreateFilesystemAncestors)?;

                let include = vkinclude::VolkanicInclude::new().await;

                let p = match include.get(id).await {
                    Some(p) => p,
                    None => return Err(JobError::NotAvailableInIncludeFolder(id.to_string())),
                };

                fs::copy(p, store.build_path.join(template_path))
                    .await
                    .map_err(JobError::Filesystem)?;
            }
            JobAction::ProcessVariables {
                path,
                format,
                variables,
            } => {
                let mut contents = fs::read_to_string(store.build_path.join(path))
                    .await
                    .map_err(JobError::Filesystem)?;

                contents = template::var::string_replace(contents, variables, format.clone()).await;

                let mut f = fs::File::create(store.build_path.join(path))
                    .await
                    .map_err(JobError::Filesystem)?;

                f.write_all(contents.as_bytes())
                    .await
                    .map_err(JobError::Filesystem)?;
            }
            JobAction::PrepareJdk { jdk, no_verify } => {
                prepare_jdk::prepare_jdk(store.clone(), jdk.clone(), *no_verify)
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
    var_map: &template::var::VarMap,
    no_verify: bool,
) -> Result<Vec<Job>, JobError> {
    let mut jobs = vec![];

    // Setup JDK
    match &template.runtime {
        template::resource::ServerRuntimeResource::Jdk {
            version,
            additional_args: _,
        } => {
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
                    no_verify,
                },
            });
        }
    }

    // Setup server software
    match &template.server {
        template::resource::ServerExecResource::Java {
            url,
            sha512,
            args: _,
        } => {
            jobs.push(Job {
                title: "Download server software".into(),
                action: JobAction::WriteFileRemote {
                    path: resources::conf::SERVER_SOFTWARE_FILE.into(),
                    url: url.clone(),
                    user_agent: None,
                    override_name: None,
                    archive: None,
                    sha512: if no_verify {
                        None
                    } else {
                        Some(sha512.clone())
                    },
                },
            });
        }
    }

    // Setup additional resources
    for resource in &template.resources {
        match resource {
            template::resource::GenericResource::Remote {
                url,
                user_agent,
                override_name,
                sha512,
                use_variables,
                archive,
                template_path: path,
            } => {
                jobs.push(Job {
                    title: "Download additional resource".into(),
                    action: JobAction::WriteFileRemote {
                        path: path.clone(),
                        url: url.clone(),
                        user_agent: user_agent.clone(),
                        override_name: override_name.clone(),
                        archive: archive.clone(),
                        sha512: sha512.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    if archive.is_some() {
                        error!("Variable substitution is not supported for archives");

                        return Err(JobError::ArchivesCannotHaveVariables(
                            override_name
                                .clone()
                                .unwrap_or(path.clone().to_string_lossy().to_string())
                                .into(),
                        ));
                    }

                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
            }
            template::resource::GenericResource::Base64 {
                base64: base,
                use_variables,
                template_path,
            } => {
                jobs.push(Job {
                    title: "Write file from Base64".into(),
                    action: JobAction::WriteFileBase64 {
                        path: template_path.clone(),
                        contents: base.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: template_path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
            }
            template::resource::GenericResource::Include {
                include_id,
                use_variables,
                template_path,
            } => {
                jobs.push(Job {
                    title: "Copy additional resource".into(),
                    action: JobAction::CopyFromInclude {
                        id: include_id.to_string(),
                        template_path: template_path.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: template_path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
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
