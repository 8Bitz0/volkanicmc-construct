use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::resource::{GenericResource, ServerRuntimeResource};

pub mod parse;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Overlay {
    #[serde(rename = "template-format")]
    pub template_format: usize,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: Option<(u64, Option<u64>, Option<u64>)>,
    pub runtime: Option<ServerRuntimeResource>,
    pub resources: Vec<GenericResource>,
    pub saveables: Vec<PathBuf>,
}

impl Overlay {
    pub async fn import<P: AsRef<Path>>(file: P) -> Result<Self, parse::Error> {
        parse::file_to_template(file.as_ref()).await
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            template_format: super::TEMPLATE_FORMAT,
            name: "Blank Overlay".into(),
            description: "Overlay that does nothing".into(),
            author: Some("Example".into()),
            version: Some((1, Some(0), Some(0))),
            runtime: None,
            resources: vec![],
            saveables: vec![],
        }
    }
}
