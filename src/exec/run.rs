use std::{path, process};
use tracing::{debug, error, info};

use crate::{
    build::{BuildInfo, BuildInfoError},
    hostinfo,
    vkstore::VolkanicStore,
};

#[cfg(target_os = "windows")]
use crate::resources;

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

// Prevent "dead_code" warning when compiling on non-Windows targets
#[allow(dead_code)]
/// Removes that weird "\\?\" prefix from Windows paths
///
/// Hacky solution for a hacky operating system.
pub async fn winpath_fix(path: impl std::fmt::Display) -> String {
    path.to_string().replace("\\\\?\\", "")
}

pub async fn run(store: &VolkanicStore) -> Result<(), ExecutionError> {
    // Check if build information exists
    if !BuildInfo::exists(store).await {
        error!("There's no build in the current directory!");
        return Err(ExecutionError::BuildNotFound);
    }

    // Parse build information
    let build_info = BuildInfo::get(store)
        .await
        .map_err(ExecutionError::BuildInfoRetrieval)?;

    let exec_info = build_info.exec.ok_or(ExecutionError::BuildNotFound)?;

    // Check if the build info requirements matches the host
    let arch = if let Some(a) = hostinfo::Arch::get().await {
        a
    } else {
        Err(ExecutionError::UnknownArchitecture)?
    };
    let os = if let Some(a) = hostinfo::Os::get().await {
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

    // Check if the build directory exists
    if !store.build_path.is_dir() {
        error!("Build directory is not present");
        return Err(ExecutionError::BuildNotFound);
    }

    // Check if the executable exists
    if !exec_info.exec_path.is_file() {
        error!(
            "Runtime executable does not exist at path: {}",
            exec_info.exec_path.display()
        );
        return Err(ExecutionError::RuntimeExecNotFound(
            exec_info.exec_path.clone(),
        ));
    }

    // Get the absolute path of the executable. This is required as using the relative
    // path to the executable will stop working after changing the current directory.
    let full_runtime_exec_path = exec_info
        .exec_path
        .canonicalize()
        .map_err(|_| ExecutionError::PathCanonicalizationFailed(exec_info.exec_path.clone()))?;

    debug!(
        "Absolute path to executable: {}",
        full_runtime_exec_path.display()
    );

    let command: (String, Vec<String>) = {
        #[cfg(not(target_os = "windows"))]
        {
            let mut runtime_args = vec![];
            runtime_args.extend(exec_info.args.clone());

            (
                full_runtime_exec_path.to_string_lossy().to_string(),
                runtime_args,
            )
        }
        #[cfg(target_os = "windows")]
        {
            let mut runtime_args = vec![];
            // Tells `cmd.exe` to execute a command
            runtime_args.push("/C".to_string());
            runtime_args
                .push(winpath_fix(full_runtime_exec_path.to_string_lossy().to_string()).await);
            runtime_args.extend(exec_info.args.clone());

            (resources::conf::WIN_SHELL_CMD.to_string(), runtime_args)
        }
    };

    debug!(
        "Executing command: \"{} {}\"",
        command.0,
        command.1.join(" "),
    );

    let mut server_proc = process::Command::new(command.0)
        .args(command.1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_winpath_fix() {
        assert_eq!(
            winpath_fix("\\\\?\\C:\\Windows\\System32\\cmd.exe".to_string()).await,
            "C:\\Windows\\System32\\cmd.exe"
        )
    }
}
