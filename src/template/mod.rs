use serde::{Deserialize, Serialize};
use std::path;

mod parse;
pub mod resource;

pub use parse::ParseError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Template {
    /// Name of the template. The name should briefly describe and identify the template.
    /// 
    /// Example: "1.12.2 Vanilla"
    pub name: String,
    /// Longer description of the template.
    /// 
    /// Example: "Server running vanilla Minecraft 1.12.2"
    pub description: String,
    /// Simple identifier of the author.
    pub author: Option<String>,
    /// Version of the template.
    pub version: Option<(u64, Option<u64>, Option<u64>)>,
    /// Server runtime software.
    pub runtime: resource::ServerRuntimeResource,
    /// Server software resource.
    pub server: resource::ServerExecResource,
    /// List of additional resources (e.g. plugins, mods, configs, etc.)
    pub resources: Vec<resource::GenericResource>,
}

impl Template {
    pub async fn import(file: path::PathBuf) -> Result<Self, ParseError> {
        parse::json_to_template(path::PathBuf::from(file)).await
    }
}

impl std::default::Default for Template {
    fn default() -> Self {
        Self {
            name: "1.20.2 Paper".into(),
            description: "Server running Minecraft 1.20.2 with PaperMC".into(),
            author: Some("Example".into()),
            version: Some((1, Some(0), Some(0))),
            runtime: resource::ServerRuntimeResource::Jdk { version: "17".to_string() },
            server: resource::ServerExecResource::Java {
                url: "https://api.papermc.io/v2/projects/paper/versions/1.20.2/builds/291/downloads/paper-1.20.2-291.jar".into(),
                sha512: "6179a94b15cbfd141431e509806ab5ce04655effea9866a5a33673b82e7fffe6fb438147565b73c98140e5cf1a5b7d9b083978c46d5239fd08b26863c423a820".into(),
            },
            resources: vec![],
        }
    }
}
