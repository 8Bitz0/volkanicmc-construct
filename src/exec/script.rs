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
                "{}\n\nexport JDK_PATH=$(realpath {})\ncd {} && exec $JDK_PATH {}",
                BASH_SHEBANG,
                exec_info.exec_path.to_string_lossy(),
                build_path.as_ref().to_string_lossy(),
                exec_info.args.join(" "),
            )
        }
        ExecScriptType::Batch => {
            format!(
                "{}\n\nset \"JDK_PATH=%~dp0\\{}\"\ncd {}\n\"%JDK_PATH%\" {}",
                BATCH_ECHO_OFF,
                exec_info.exec_path.to_string_lossy(),
                build_path.as_ref().to_string_lossy(),
                exec_info.args.join(" "),
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
            exec_path: std::path::PathBuf::from(".volkanic/runtime/java"),
            args: vec![
                "-Xms512M".to_string(),
                "-Xmx1024M".to_string(),
                "-jar".to_string(),
                "server.jar".to_string(),
            ],
        };

        let script = to_script(
            exec_info,
            std::path::PathBuf::from(".volkanic/build"),
            crate::exec::script::ExecScriptType::Bash,
        )
        .await;

        assert_eq!(script, "#!/usr/bin/env bash\n\nexport JDK_PATH=$(realpath .volkanic/runtime/java)\ncd .volkanic/build && exec $JDK_PATH -Xms512M -Xmx1024M -jar server.jar");
    }
}
