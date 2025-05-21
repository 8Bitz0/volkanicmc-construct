use serde::{Deserialize, Serialize};
use std::path;
use tokio::fs;
use tracing::error;

use crate::fsobj;
use crate::resources;
use crate::resources::Jdk;
use crate::template;
use crate::vkstore;

use super::buildinfo;
use super::misc;
use super::prepare_jdk;

mod copy_include;
mod create_jobs;
mod process_vars;
mod write_base;
mod write_remote;

pub use create_jobs::create_jobs;

use copy_include::copy_include;
use process_vars::process_vars;
use write_base::write_base64;
use write_remote::write_remote;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No JDK found for your system (version: {0})")]
    JdkNotFound(String),
    #[error("Foojay Disco lookup error: {0}")]
    DiscoLookup(resources::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
    #[error("Inner archive path doesn't exist: {0}")]
    InnerArchivePathNotFound(path::PathBuf),
    #[error("Extraction error: {0}")]
    Extraction(misc::ExtractionError),
    #[error("No file name found in path: {0}")]
    NoFileNameInPath(path::PathBuf),
    #[error("Creating path ancestor directories failed: {0}")]
    CreateFilesystemAncestors(fsobj::CreateAncestorError),
    #[error("Directory copy failed: {0}")]
    DirectoryCopyFailed(path::PathBuf),
    #[error("Base64 error: {0}")]
    Base64(base64::DecodeError),
    #[error("Download error: {0}")]
    Download(misc::DownloadError),
    #[error("Prepare JDK error: {0}")]
    PrepareJdk(prepare_jdk::Error),
    #[error("Build info error: {0}")]
    BuildInfo(buildinfo::BuildInfoError),
    #[error("Not available in Volkanic include folder: {0}")]
    NotAvailableInIncludeFolder(String),
    #[error("Archives cannot have variables (resource path: {0})")]
    ArchivesCannotHaveVariables(path::PathBuf),
    #[error("Conflicting overlay runtimes")]
    ConflictingRuntimes,
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
    pub async fn execute(&self, store: &vkstore::VolkanicStore) -> Result<(), Error> {
        match self {
            JobAction::CreateDir {
                path: template_path,
            } => {
                fs::create_dir_all(store.build_path.join(template_path))
                    .await
                    .map_err(Error::Filesystem)?;
            }
            JobAction::WriteFileBase64 {
                path: template_path,
                contents,
            } => {
                write_base64(store, template_path, contents).await?;
            }
            JobAction::WriteFileRemote {
                path: template_path,
                archive,
                url,
                sha512,
                user_agent,
                override_name,
            } => {
                write_remote(
                    store,
                    template_path,
                    archive.as_ref(),
                    url,
                    sha512.as_ref(),
                    user_agent.as_ref(),
                    override_name.as_ref(),
                )
                .await?;
            }
            JobAction::CopyFromInclude { id, template_path } => {
                copy_include(store, id, template_path).await?;
            }
            JobAction::ProcessVariables {
                path,
                format,
                variables,
            } => {
                process_vars(store, format.clone(), path, variables).await?;
            }
            JobAction::PrepareJdk { jdk, no_verify } => {
                prepare_jdk::prepare_jdk(store.clone(), jdk.clone(), *no_verify)
                    .await
                    .map_err(Error::PrepareJdk)?;
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

pub async fn execute_jobs(
    store: vkstore::VolkanicStore,
    build_info: &mut buildinfo::BuildInfo,
) -> Result<(), Error> {
    build_info.job_progress = 0;

    for job in &build_info.jobs {
        job.action.execute(&store).await?;

        build_info.job_progress += 1;

        build_info.update().await.map_err(Error::BuildInfo)?;
    }

    Ok(())
}
