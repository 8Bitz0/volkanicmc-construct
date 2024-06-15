use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::template::var::{string_replace, VarFormat};
use crate::vkstore::VolkanicStore;

use super::JobError;

pub async fn process_vars<P: AsRef<Path>>(
    store: &VolkanicStore,
    format: VarFormat,
    template_path: P,
    variables: &HashMap<String, String>,
) -> Result<(), JobError> {
    let abs_path = store.build_path.join(template_path.as_ref());

    let mut contents = fs::read_to_string(&abs_path)
        .await
        .map_err(JobError::Filesystem)?;

    contents = string_replace(contents, variables, format.clone()).await;

    let mut f = fs::File::create(&abs_path)
        .await
        .map_err(JobError::Filesystem)?;

    f.write_all(contents.as_bytes())
        .await
        .map_err(JobError::Filesystem)?;

    Ok(())
}
