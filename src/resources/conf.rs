pub const FILE_BUFFER_SIZE: usize = 1024;

#[cfg(not(target_os = "windows"))]
pub const JDK_BIN_FILE: &str = "bin/java";
#[cfg(target_os = "windows")]
pub const JDK_BIN_FILE: &str = "bin/java.exe";

#[cfg(target_os = "windows")]
pub const WIN_SHELL_CMD: &str = "cmd.exe";
