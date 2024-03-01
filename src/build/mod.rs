use tracing::{debug, error, info, warn};

mod buildinfo;
mod job;
mod misc;
mod prepare_jdk;

use crate::exec;
use crate::hostinfo;
use crate::resources::{self, JdkConfig};
use crate::template;
use crate::vkstore;

pub use buildinfo::{BuildInfo, BuildInfoError};

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Build info error: {0}")]
    BuildInfo(buildinfo::BuildInfoError),
    #[error("Unknown platform")]
    UnknownPlatform,
    #[error("Unknown architecture")]
    UnknownArchitecture,
    #[error("Job error: {0}")]
    Job(job::JobError),
    #[error("Resource error: {0}")]
    ResourceLoad(resources::ResourceLoadError),
    #[error("Store error: {0}")]
    Store(vkstore::StoreError),
    #[error("Build is already present")]
    BuildPresent,
    #[error("Variable processing error: {0}")]
    VarProcess(template::var::VarProcessError),
}

pub async fn build(
    template: template::Template,
    store: vkstore::VolkanicStore,
    force: bool,
    user_vars_raw: Vec<String>,
) -> Result<(), BuildError> {
    let mut user_vars = template::var::EnvMap::new();

    for var in user_vars_raw {
        let mut split = var.splitn(2, '=');
        let name = split
            .next()
            .ok_or(BuildError::VarProcess(
                template::var::VarProcessError::RawVarWithoutName,
            ))?
            .to_string();
        let value = split
            .next()
            .ok_or(BuildError::VarProcess(
                template::var::VarProcessError::RawVarWithoutValue,
            ))?
            .to_string();
        user_vars.insert(name, value);
    }

    let jdk_config = JdkConfig::parse_list()
        .await
        .map_err(BuildError::ResourceLoad)?;

    info!("Creating template variables...");
    let mut variables = template::var::VarMap::new();

    template::var::process_vars(&mut variables, template.variables.clone(), &user_vars)
        .map_err(BuildError::VarProcess)?;

    info!("Creating jobs...");
    let jobs = job::create_jobs(&template, jdk_config, &variables)
        .await
        .map_err(BuildError::Job)?;

    info!("Scheduled {} jobs", jobs.len());

    let mut build_info = {
        if buildinfo::BuildInfo::exists(&store).await {
            let mut build_info = buildinfo::BuildInfo::get(&store)
                .await
                .map_err(BuildError::BuildInfo)?;

            if build_info.job_progress == build_info.jobs.len() && !build_info.jobs.is_empty() {
                warn!("Build is already present.");

                if force {
                    warn!("Rebuild forced")
                } else {
                    error!("Please specify the \"--force\" flag to rebuild.");
                    return Err(BuildError::BuildPresent);
                }
            } else {
                warn!("Incomplete build found. Rebuilding...");
            }

            build_info.jobs = jobs;
            store.renew().await.map_err(BuildError::Store)?;

            build_info.set_path(&store);

            build_info
        } else {
            let mut build_info = buildinfo::BuildInfo::new(&store)
                .await
                .map_err(BuildError::BuildInfo)?;

            build_info.jobs = jobs;

            store.renew().await.map_err(BuildError::Store)?;

            build_info
        }
    };

    let server_args: Option<Vec<String>> = match &template.server {
        template::resource::ServerExecResource::Java {
            url: _,
            sha512: _,
            args: params,
        } => Some(params.split(' ').map(|s| s.to_string()).collect()),
    };

    job::execute_jobs(store.clone(), &mut build_info)
        .await
        .map_err(BuildError::Job)?;

    store.clean().await.map_err(BuildError::Store)?;

    debug!("Setting build execution info");

    build_info.exec = Some(exec::BuildExecInfo {
        arch: if let Some(a) = hostinfo::Arch::get() {
            a
        } else {
            return Err(BuildError::UnknownArchitecture);
        },
        os: if let Some(a) = hostinfo::Os::get() {
            a
        } else {
            return Err(BuildError::UnknownPlatform);
        },
        runtime_args: match &template.runtime {
            template::resource::ServerRuntimeResource::Jdk {
                version: _,
                additional_args,
            } => match additional_args.clone() {
                Some(args) => {
                    warn!(
                        "Additional JDK arguments are in use (\"{}\")",
                        args.join(" ")
                    );
                    args
                }
                None => vec![],
            },
        },
        runtime_exec_path: store.runtime_path.join(resources::conf::JDK_BIN_FILE),
        server_args: match server_args {
            Some(args) => args,
            None => vec![],
        },
        server_jar_path: resources::conf::SERVER_SOFTWARE_FILE.into(),
    });

    build_info.update().await.map_err(BuildError::BuildInfo)?;

    info!("Build complete");

    Ok(())
}
