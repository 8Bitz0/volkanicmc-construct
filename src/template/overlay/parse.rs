use std::path;
use tokio::fs;

use super::Overlay;

pub const FORMAT_ENTRY: &str = "template-format";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to parse JSON: {0}")]
    JsonParse(serde_jsonc::Error),
    #[error("Template format error: {0}")]
    Format(String),
    #[error("Filesystem error: {0}")]
    Filesystem(tokio::io::Error),
}

pub async fn file_to_template<P: AsRef<path::Path>>(path: P) -> Result<Overlay, Error> {
    let json = fs::read_to_string(&path)
        .await
        .map_err(Error::Filesystem)?;

    json_to_template(&json).await
}

pub async fn json_to_template(json: impl std::fmt::Display) -> Result<Overlay, Error> {
    let json_value: serde_jsonc::Value =
        serde_jsonc::from_str(&json.to_string()).map_err(Error::JsonParse)?;

    let template_format = json_value[FORMAT_ENTRY].as_u64();
    // Check that the template format value is an integer and then compare
    // against the template format constant.
    if let Some(template_format) = template_format {
        if template_format != super::super::TEMPLATE_FORMAT as u64 {
            return Err(Error::Format(format!(
                "Template format mismatch (found format version \'{}\', only \'{}\' is supported)",
                template_format,
                super::super::TEMPLATE_FORMAT
            )));
        }
    } else if template_format != Some(super::super::TEMPLATE_FORMAT as u64) {
        return Err(Error::Format("Not a valid template".into()));
    }

    serde_jsonc::from_str(&json.to_string()).map_err(Error::JsonParse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_string_to_template() {
        let template = Overlay::default();

        let template_str = serde_jsonc::to_string(&template).unwrap();

        json_to_template(template_str).await.unwrap();
    }
}
