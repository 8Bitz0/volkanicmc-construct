use serde::{Deserialize, Serialize};
use std::path;

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
    Java { url: String, sha512: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GenericResource {
    /// A remote file to download via provided URL
    #[serde(rename = "remote")]
    Remote { url: String, sha512: Option<String>, path: String },
    /// Copy file to template
    #[serde(rename = "fs_copy")]
    FsCopy { path: path::PathBuf, template_path: path::PathBuf },
    /// Modrinth mod
    #[serde(rename = "modrinth")]
    Modrinth { identity: ModrinthProject },
}
