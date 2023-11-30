use std::path;
use tokio::fs;

use super::Template;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Failed to parse JSON: {0}")]
    JsonParse(serde_jsonc::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
}

pub async fn json_to_template(path: path::PathBuf) -> Result<Template, ParseError> {
    let json = fs::read_to_string(path)
        .await
        .map_err(ParseError::Filesystem)?;

    serde_jsonc::from_str(&json).map_err(ParseError::JsonParse)
}
