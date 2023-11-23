use tracing::info;

mod job;
mod lock;
mod misc;
mod prepare_jdk;

use crate::resources;
use crate::resources::JdkConfig;
use crate::template;
use crate::vkstore;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Lock error: {0}")]
    LockError(lock::LockError),
    #[error("Job error: {0}")]
    JobError(job::JobError),
    #[error("Resource error: {0}")]
    ResourceLoadError(resources::ResourceLoadError),
}

pub async fn build(template: template::Template, store: vkstore::VolkanicStore) -> Result<(), BuildError> {
    let jdk_config = JdkConfig::parse_list().await.map_err(BuildError::ResourceLoadError)?;

    info!("Creating jobs...");
    let mut jobs = job::create_jobs(&template, jdk_config).await.map_err(BuildError::JobError)?;

    info!("Scheduled {} jobs.", jobs.len());

    let lock = lock::Lock::new(&store, jobs).await.map_err(BuildError::LockError)?;

    Ok(())
}
