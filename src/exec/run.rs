use std::{path, process};
use tracing::{debug, error, info};

use crate::{
    build::{BuildInfo, BuildInfoError},
    hostinfo,
    vkstore::VolkanicStore,
};

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Build not found")]
    BuildNotFound,
    #[error("Runtime execuatable does not exist at path: {0}")]
    RuntimeExecNotFound(path::PathBuf),
    #[error("Failed to retrieve build info: {0}")]
    BuildInfoRetrieval(BuildInfoError),
    #[error("Unknown platform")]
    UnknownPlatform,
    #[error("Unknown architecture")]
    UnknownArchitecture,
    #[error("Path canonicalization failed: {0}")]
    PathCanonicalizationFailed(path::PathBuf),
    #[error("Child process failed to spawn: {0}")]
    ChildProcessSpawnFailed(std::io::Error),
    #[error("Child process failed")]
    ChildProcessFailed,
    #[error("Child process closed with error code: {0}")]
    ChildProcessFailedCode(i32),
}

pub async fn run(store: &VolkanicStore) -> Result<(), ExecutionError> {
    let build_info = BuildInfo::get(store)
        .await
        .map_err(ExecutionError::BuildInfoRetrieval)?;

    let exec_info = build_info.exec.ok_or(ExecutionError::BuildNotFound)?;

    let arch = if let Some(a) = hostinfo::Arch::get() {
        a
    } else {
        Err(ExecutionError::UnknownArchitecture)?
    };
    let os = if let Some(a) = hostinfo::Os::get() {
        a
    } else {
        Err(ExecutionError::UnknownPlatform)?
    };

    if exec_info.arch != arch {
        error!(
            "This configuration expects architecture: \"{:?}\", but running on: \"{:?}\"",
            exec_info.arch, arch
        );
        return Err(ExecutionError::UnknownArchitecture);
    }

    if exec_info.os != os {
        error!(
            "This configuration expects platform: \"{:?}\", but running on: \"{:?}\"",
            exec_info.os, os
        );
        return Err(ExecutionError::UnknownArchitecture);
    }

    if !store.build_path.is_dir() {
        error!("Build directory is not present");
        return Err(ExecutionError::BuildNotFound);
    }

    if !exec_info.runtime_exec_path.is_file() {
        error!(
            "Runtime executable does not exist at path: {}",
            exec_info.runtime_exec_path.display()
        );
        return Err(ExecutionError::RuntimeExecNotFound(
            exec_info.runtime_exec_path.clone(),
        ));
    }

    let full_runtime_exec_path = exec_info.runtime_exec_path.canonicalize().map_err(|_| {
        ExecutionError::PathCanonicalizationFailed(exec_info.runtime_exec_path.clone())
    })?;

    let full_server_exec_path = store
        .build_path
        .join(&exec_info.server_jar_path)
        .canonicalize()
        .map_err(|_| {
            ExecutionError::PathCanonicalizationFailed(exec_info.server_jar_path.clone())
        })?;

    debug!(
        "Absolute path to runtime executable: {}",
        full_runtime_exec_path.display()
    );
    debug!(
        "Absolute path to server executable: {}",
        full_server_exec_path.display()
    );

    let mut runtime_args: Vec<String> = vec![];
    runtime_args.extend(exec_info.runtime_args.clone());
    runtime_args.push("-jar".to_string());
    runtime_args.push(full_server_exec_path.to_string_lossy().to_string());
    runtime_args.extend(exec_info.server_args);

    debug!("Spawning child process with arguments: {:?}", runtime_args);

    debug!(
        "Executing command: \"{} {}\"",
        full_runtime_exec_path.display(),
        runtime_args.join(" ")
    );

    let mut server_proc = process::Command::new(full_runtime_exec_path)
        .args(runtime_args)
        .current_dir(&store.build_path)
        .spawn()
        .map_err(ExecutionError::ChildProcessSpawnFailed)?;

    info!("Spawned server process");

    let exit_status = server_proc
        .wait()
        .map_err(ExecutionError::ChildProcessSpawnFailed)?;

    if !exit_status.success() {
        match exit_status.code() {
            Some(code) => {
                error!("Child process failed with error code: {}", code);
                return Err(ExecutionError::ChildProcessFailedCode(code));
            }
            None => {
                error!("Child process failed");
                return Err(ExecutionError::ChildProcessFailed);
            }
        }
    }

    Ok(())
}
