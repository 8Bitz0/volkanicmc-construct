use std::fmt::Display;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[allow(clippy::enum_variant_names)]
pub enum Os {
    #[serde(rename = "freebsd")]
    FreeBsd,
    #[serde(rename = "macos")]
    MacOs,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "alpine")]
    Alpine,
    #[serde(rename = "linux")]
    Linux,
}

impl<'de> Deserialize<'de> for Os {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "freebsd" => Ok(Os::FreeBsd),
            "macos" => Ok(Os::MacOs),
            "windows" => Ok(Os::Windows),
            "alpine" => Ok(Os::Alpine),
            "linux" => Ok(Os::Linux),
            _ => Err(serde::de::Error::custom(format!("Invalid OS: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Arch {
    #[serde(rename = "x86")]
    X86,
    #[serde(rename = "amd64")]
    Amd64,
    #[serde(rename = "arm")]
    Arm,
    #[serde(rename = "arm64")]
    Arm64,
    #[serde(rename = "ppc")]
    PowerPc,
    #[serde(rename = "ppc64")]
    PowerPc64,
    #[serde(rename = "riscv64")]
    RiscV64,
}

impl<'de> Deserialize<'de> for Arch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "x86" => Ok(Arch::X86),
            "amd64" => Ok(Arch::Amd64),
            "arm" => Ok(Arch::Arm),
            "arm64" => Ok(Arch::Arm64),
            "ppc" => Ok(Arch::PowerPc),
            "ppc64" => Ok(Arch::PowerPc64),
            "riscv64" => Ok(Arch::RiscV64),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid architecture: {}",
                s
            ))),
        }
    }
}

impl Os {
    pub async fn get() -> Option<Os> {
        Some(match std::env::consts::OS {
            "freebsd" => Os::FreeBsd,
            "macos" => Os::MacOs,
            "windows" => Os::Windows,
            "linux" => match sysinfo::System::distribution_id().as_str() {
                "alpine" => Os::Alpine,
                _ => Os::Linux,
            },
            _ => return None,
        })
    }
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Os::FreeBsd => "freebsd",
            Os::MacOs => "macos",
            Os::Windows => "windows",
            Os::Linux => "linux",
            Os::Alpine => "alpine",
        })
    }
}

impl Arch {
    pub async fn get() -> Option<Arch> {
        Some(match std::env::consts::ARCH {
            "x86" => Arch::X86,
            "x86_64" => Arch::Amd64,
            "arm" => Arch::Arm,
            "aarch64" => Arch::Arm64,
            "powerpc" => Arch::PowerPc,
            "powerpc64" => Arch::PowerPc64,
            "riscv64" => Arch::RiscV64,
            _ => return None,
        })
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Arch::X86 => "x86",
            Arch::Amd64 => "amd64",
            Arch::Arm => "arm",
            Arch::Arm64 => "arm64",
            Arch::PowerPc => "ppc",
            Arch::PowerPc64 => "ppc64",
            Arch::RiscV64 => "riscv64"
        })
    }
}
