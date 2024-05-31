use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path;

use crate::resources;

use super::var::VarFormat;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum ServerRuntimeResource {
    #[serde(rename = "jdk")]
    Jdk {
        version: String,
        /// Path to JAR executable
        #[serde(rename = "jar-path")]
        jar_path: path::PathBuf,
        /// Adds additional JDK arguments
        #[serde(rename = "jdk-args")]
        jdk_args: Vec<String>,
        /// Adds additional arguments for JAR executable
        #[serde(rename = "server-args")]
        server_args: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct ArchiveInfo {
    #[serde(rename = "internal-path")]
    pub inner_path: path::PathBuf,
    #[serde(rename = "format")]
    pub archive_format: resources::ArchiveFormat,
    /// Paths to remove after extraction (relative to the new directory)
    #[serde(rename = "post-remove")]
    pub post_remove: Vec<path::PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum GenericResource {
    /// A remote file to download via provided URL
    #[serde(rename = "remote")]
    Remote {
        /// URL of the remote file
        url: String,
        /// Custom user agent to use for the download
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "user-agent")]
        user_agent: Option<String>,
        /// Optional name of the remote file
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "override-name")]
        override_name: Option<String>,
        /// Optional SHA-512 hash of the remote file for verification
        #[serde(skip_serializing_if = "Option::is_none")]
        sha512: Option<String>,
        /// Whether to use variables in the file
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "use-variables")]
        use_variables: Option<VarFormat>,
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
        /// Whether to use variables in the file
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "use-variables")]
        use_variables: Option<VarFormat>,
        /// Path the file should be written to inside the build
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
    /// Copy file from Volkanic include folder to template
    #[serde(rename = "include")]
    Include {
        #[serde(rename = "id")]
        include_id: String,
        /// Whether to use variables in the file
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "use-variables")]
        use_variables: Option<VarFormat>,
        /// Path the file should be written to inside the build
        #[serde(rename = "template-path")]
        template_path: path::PathBuf,
    },
}
