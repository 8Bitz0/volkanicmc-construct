use base64::Engine;
use std::path::Path;
use tokio::fs;
use tracing::error;

use crate::{fsobj, vkstore::VolkanicStore};

use super::JobError;

pub async fn write_base64<P: AsRef<Path>, T: std::fmt::Display>(
    store: &VolkanicStore,
    template_path: P,
    contents: T,
) -> Result<(), JobError> {
    let abs_path = store.build_path.join(template_path.as_ref());

    fsobj::create_ancestors(&abs_path)
        .await
        .map_err(JobError::CreateFilesystemAncestors)?;

    let base64_config = base64::engine::GeneralPurposeConfig::new();
    let base64_engine =
        base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64_config);

    let contents = base64_engine.decode(contents.to_string()).map_err(|err| {
        error!("Failed to decode base64: {}", err);
        JobError::Base64(err)
    })?;

    fs::write(&abs_path, contents)
        .await
        .map_err(JobError::Filesystem)?;

    Ok(())
}
