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
    BuildInfo(buildinfo::BuildInfoError),
    #[error("Job error: {0}")]
    Job(job::JobError),
    #[error("Resource error: {0}")]
    ResourceLoad(resources::ResourceLoadError),
    #[error("Store error: {0}")]
    Store(vkstore::StoreError),
    #[error("Build is already present")]
    BuildPresent,
}

pub async fn build(template: template::Template, store: vkstore::VolkanicStore, force: bool) -> Result<(), BuildError> {
    let jdk_config = JdkConfig::parse_list().await.map_err(BuildError::ResourceLoad)?;

    info!("Creating jobs...");
    let jobs = job::create_jobs(&template, jdk_config).await.map_err(BuildError::Job)?;

    info!("Scheduled {} jobs", jobs.len());

    let mut build_info = {
        if buildinfo::BuildInfo::exists(&store).await {
            let mut build_info = buildinfo::BuildInfo::get(&store).await.map_err(BuildError::BuildInfo)?;

            if build_info.jobs != jobs {
                error!("Build is already present but template has changed. Use \"--force\" to override.");
                return Err(BuildError::BuildPresent);
            }

            if build_info.job_progress == build_info.jobs.len() {
                if force {
                    warn!("Build is already present. Rebuild has been forced.");

                    store.renew().await.map_err(BuildError::Store)?;
                } else {
                    error!("Build is already present. Use \"--force\" to override.");
                    return Err(BuildError::BuildPresent);
                }
            }

            build_info.set_path(&store);

            build_info
        } else {
            let mut build_info = buildinfo::BuildInfo::new(&store).await.map_err(BuildError::BuildInfo)?;

            build_info.jobs = jobs;

            store.renew().await.map_err(BuildError::Store)?;

            build_info
        }
    };

    job::execute_jobs(store.clone(), &mut build_info).await.map_err(BuildError::Job)?;

    store.clean().await.map_err(BuildError::Store)?;

    info!("Build complete");

    Ok(())
}
