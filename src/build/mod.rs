use tracing::{info, error};  

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
    #[error("Store error: {0}")]
    StoreError(vkstore::StoreError),
    #[error("Build is already present")]
    BuildPresent,
}

pub async fn build(template: template::Template, store: vkstore::VolkanicStore, force: bool) -> Result<(), BuildError> {
    if store.build_path.is_dir() || store.runtime_path.is_dir() {
        if force {
            store.renew().await.map_err(BuildError::StoreError)?;
        } else {
            error!("Build is already present. Use \"--force\" to override.");
            return Err(BuildError::BuildPresent);
        }
    }

    let jdk_config = JdkConfig::parse_list().await.map_err(BuildError::ResourceLoadError)?;

    info!("Creating jobs...");
    let jobs = job::create_jobs(&template, jdk_config).await.map_err(BuildError::JobError)?;

    let mut lock = lock::Lock::new(&store, jobs.clone()).await.map_err(BuildError::LockError)?;

    info!("Scheduled {} jobs", jobs.len());

    if !lock.stray {
        store.renew().await.map_err(BuildError::StoreError)?;
    }
    

    job::execute_jobs(store.clone(), &mut lock).await.map_err(BuildError::JobError)?;

    lock.remove().await.map_err(BuildError::LockError)?;
    store.clean().await.map_err(BuildError::StoreError)?;

    info!("Build complete");

    Ok(())
}
