use tracing::{info, warn, error};  

mod job;
mod buildinfo;
mod misc;
mod prepare_jdk;

use crate::resources::{self, JdkConfig};
use crate::template;
use crate::vkstore;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Build info error: {0}")]
    BuildInfoError(buildinfo::BuildInfoError),
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
    let jdk_config = JdkConfig::parse_list().await.map_err(BuildError::ResourceLoadError)?;

    info!("Creating jobs...");
    let jobs = job::create_jobs(&template, jdk_config).await.map_err(BuildError::JobError)?;

    info!("Scheduled {} jobs", jobs.len());

    let mut build_info = {
        if buildinfo::BuildInfo::exists(&store).await {
            let mut build_info = buildinfo::BuildInfo::get(&store).await.map_err(BuildError::BuildInfoError)?;

            if build_info.jobs != jobs {
                warn!("Build is already present but template has changed. Use \"--force\" to override.");
                return Err(BuildError::BuildPresent);
            }

            if build_info.job_progress == build_info.jobs.len() {
                if force {
                    warn!("Build is already present. Overriding...");

                    store.renew().await.map_err(BuildError::StoreError)?;
                } else {
                    error!("Build is already present. Use \"--force\" to override.");
                    return Err(BuildError::BuildPresent);
                }
            }

            build_info.set_path(&store);

            build_info
        } else {
            let mut build_info = buildinfo::BuildInfo::new(&store).await.map_err(BuildError::BuildInfoError)?;

            build_info.jobs = jobs;

            store.renew().await.map_err(BuildError::StoreError)?;

            build_info
        }
    };

    job::execute_jobs(store.clone(), &mut build_info).await.map_err(BuildError::JobError)?;

    store.clean().await.map_err(BuildError::StoreError)?;

    info!("Build complete");

    Ok(())
}
