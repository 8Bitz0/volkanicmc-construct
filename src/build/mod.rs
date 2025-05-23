use tracing::{debug, error, info, warn};

mod buildinfo;
mod job;
mod misc;
mod prepare_jdk;

use crate::exec;
use crate::hostinfo;
use crate::resources::{self, JdkLookup};
use crate::template::{self, overlay::Overlay};
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
    Job(job::Error),
    #[error("Store error: {0}")]
    Store(vkstore::StoreError),
    #[error("Build is already present")]
    BuildPresent,
    #[error("Variable processing error: {0}")]
    VarProcess(template::var::VarProcessError),
}

pub async fn build(
    template: template::Template,
    overlays: Vec<Overlay>,
    store: vkstore::VolkanicStore,
    force: bool,
    user_vars_raw: Vec<String>,
    additional_jvm_args: Vec<String>,
    prevent_verify: bool,
    force_jdk_distribution: Option<String>,
    preferred_distributions: Option<Vec<String>>,
) -> Result<(), BuildError> {
    let mut user_vars = template::var::EnvMap::new();

    if prevent_verify {
        warn!("Verification is disabled. Continue at your own risk.");
    }

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

    let jdk_config = JdkLookup::new();

    info!("Creating template variables...");
    let mut variables = template::var::VarMap::new();

    template::var::process_vars(&mut variables, template.variables.clone(), &user_vars)
        .await
        .map_err(BuildError::VarProcess)?;

    info!("Creating jobs...");
    let jobs = job::create_jobs(
        &template,
        &overlays,
        jdk_config,
        &variables,
        prevent_verify,
        force_jdk_distribution,
        preferred_distributions,
    )
        .await
        .map_err(BuildError::Job)?;

    info!("Scheduled {} jobs", jobs.len());

    let mut build_info = {
        if buildinfo::BuildInfo::exists(&store).await {
            let mut build_info = buildinfo::BuildInfo::get(&store)
                .await
                .map_err(BuildError::BuildInfo)?;

            warn!("Build is already present.");

            if force {
                warn!("Rebuild forced")
            } else {
                error!("Please specify the \"--force\" flag to rebuild.");
                return Err(BuildError::BuildPresent);
            }

            build_info.jobs = jobs;
            store.renew().await.map_err(BuildError::Store)?;

            build_info.set_path(&store).await;

            build_info
        } else {
            let mut build_info = buildinfo::BuildInfo::new(
                &store,
                template.clone(),
                overlays.clone(),
            )
                .await
                .map_err(BuildError::BuildInfo)?;

            build_info.jobs = jobs;

            store.renew().await.map_err(BuildError::Store)?;

            build_info
        }
    };

    job::execute_jobs(store.clone(), &mut build_info)
        .await
        .map_err(BuildError::Job)?;

    store.clean().await.map_err(BuildError::Store)?;

    debug!("Setting build execution info");

    build_info.exec = Some(match template.runtime {
        template::resource::ServerRuntimeResource::Jdk {
            version: _,
            jar_path,
            jdk_args,
            server_args,
        } => exec::BuildExecInfo {
            arch: if let Some(a) = hostinfo::Arch::get().await {
                a
            } else {
                return Err(BuildError::UnknownArchitecture);
            },
            os: if let Some(a) = hostinfo::Os::get().await {
                a
            } else {
                return Err(BuildError::UnknownPlatform);
            },
            exec_path: store.runtime_path.join(resources::conf::JDK_BIN_FILE),
            args: {
                let mut args: Vec<String> = vec![];

                args.extend(jdk_args);
                args.extend(additional_jvm_args);

                args.push("-jar".to_string());
                args.push(jar_path.to_string_lossy().to_string());
                args.push(server_args.join(" "));

                args
            },
        },
    });

    build_info.update().await.map_err(BuildError::BuildInfo)?;

    info!("Build complete");

    Ok(())
}
