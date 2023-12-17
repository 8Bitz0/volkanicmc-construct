use serde::{Deserialize, Serialize};
use std::path;

use crate::resources;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ModrinthProject {
    Id(String),
    Slug(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ServerRuntimeResource {
    #[serde(rename = "jdk")]
    Jdk { version: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ServerExecResource {
    #[serde(rename = "java")]
    Java {
        url: String,
        sha512: String,
        args: String,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ArchiveInfo {
    #[serde(rename = "internal-path")]
    pub inner_path: path::PathBuf,
    #[serde(rename = "format")]
    pub archive_format: resources::ArchiveFormat,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GenericResource {
    /// A remote file to download via provided URL
    #[serde(rename = "remote")]
    Remote {
        /// URL of the remote file
        url: String,
        /// Optional SHA-512 hash of the remote file for verification
        sha512: Option<String>,
        /// If the remote file is an archive, define the internal object to
        /// extract and the archive format
        #[serde(skip_serializing_if = "Option::is_none")]
        archive: Option<ArchiveInfo>,
        /// Path the file should be written to inside the build
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    /// A file encoded with Base64
    #[serde(rename = "base64")]
    Base64 {
        base64: String,
        /// Path the file should be written to inside the build
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    /// Copy file from Volkanic include folder to template
    #[serde(rename = "include")]
    Include {
        #[serde(rename = "id")]
        include_id: String,
        /// Path the file should be written to inside the build
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    /// Modrinth mod
    #[serde(rename = "modrinth")]
    Modrinth { identity: ModrinthProject },
}
