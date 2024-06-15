use std::path::Path;
use tokio::fs;

use crate::fsobj;
use crate::template::vkinclude;
use crate::vkstore::VolkanicStore;

use super::JobError;

pub async fn copy_include<T: std::fmt::Display, P: AsRef<Path>>(
    store: &VolkanicStore,
    id: T,
    template_path: P,
) -> Result<(), JobError> {
    let abs_path = store.build_path.join(template_path);

    fsobj::create_ancestors(abs_path.clone())
        .await
        .map_err(JobError::CreateFilesystemAncestors)?;

    let include = vkinclude::VolkanicInclude::new().await;

    let p = match include.get(&id).await {
        Some(p) => p,
        None => return Err(JobError::NotAvailableInIncludeFolder(id.to_string())),
    };

    fs::copy(p, abs_path).await.map_err(JobError::Filesystem)?;

    Ok(())
}
