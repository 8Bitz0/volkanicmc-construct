use serde::{Deserialize, Serialize};

pub mod presets;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum JdkArguments {
    #[serde(rename = "custom")]
    Custom(Vec<String>),
    #[serde(rename = "preset")]
    Preset(presets::JdkPreset),
}