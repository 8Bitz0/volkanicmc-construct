use std::path::Path;

use super::BuildExecInfo;

const BASH_SHEBANG: &str = "#!/usr/bin/env bash";
const BATCH_ECHO_OFF: &str = "@echo off";

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum ExecScriptType {
    Bash,
    Batch,
}

/// Creates a Bash script from `BuildExecInfo`
pub async fn to_script<P: AsRef<Path>>(
    exec_info: BuildExecInfo,
    build_path: P,
    format: ExecScriptType,
) -> String {
    match format {
        ExecScriptType::Bash => {
            format!(
                "{}\n\nexport JDK_PATH=$(realpath {})\ncd {} && exec $JDK_PATH {} -jar {} {}",
                BASH_SHEBANG,
                exec_info.runtime_exec_path.to_string_lossy(),
                build_path.as_ref().to_string_lossy(),
                exec_info.runtime_args.join(" "),
                exec_info.server_jar_path.to_string_lossy(),
                exec_info.server_args.join(" "),
            )
        }
        ExecScriptType::Batch => {
            format!(
                "{}\n\nset \"JDK_PATH=%~dp0\\{}\"\ncd {}\n\"%JDK_PATH%\" {} -jar {} {}",
                BATCH_ECHO_OFF,
                exec_info.runtime_exec_path.to_string_lossy(),
                build_path.as_ref().to_string_lossy(),
                exec_info.runtime_args.join(" "),
                exec_info.server_jar_path.to_string_lossy(),
                exec_info.server_args.join(" "),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::to_script;

    #[tokio::test]
    async fn test_bash() {
        let exec_info = super::BuildExecInfo {
            arch: crate::hostinfo::Arch::Amd64,
            os: crate::hostinfo::Os::Linux,
            runtime_exec_path: std::path::PathBuf::from(".volkanic/runtime/java"),
            server_jar_path: std::path::PathBuf::from("server.jar"),
            runtime_args: vec!["-Xms512M".to_string(), "-Xmx1024M".to_string()],
            server_args: vec!["-nogui".to_string()],
        };

        let script = to_script(
            exec_info,
            std::path::PathBuf::from(".volkanic/build"),
            crate::exec::script::ExecScriptType::Bash,
        )
        .await;

        assert_eq!(script, "#!/usr/bin/env bash\n\nexport JDK_PATH=$(realpath .volkanic/runtime/java)\ncd .volkanic/build && exec $JDK_PATH -Xms512M -Xmx1024M -jar server.jar -nogui");
    }
}
