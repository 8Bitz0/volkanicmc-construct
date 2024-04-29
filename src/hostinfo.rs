use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Os {
    #[serde(rename = "android")]
    Android,
    #[serde(rename = "freebsd")]
    FreeBsd,
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "mac")]
    Mac,
    #[serde(rename = "netbsd")]
    NetBsd,
    #[serde(rename = "openbsd")]
    OpenBsd,
    #[serde(rename = "dragonfly")]
    Dragonfly,
    #[serde(rename = "solaris")]
    Solaris,
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
            "android" => Ok(Os::Android),
            "freebsd" => Ok(Os::FreeBsd),
            "ios" => Ok(Os::Ios),
            "mac" => Ok(Os::Mac),
            "netbsd" => Ok(Os::NetBsd),
            "openbsd" => Ok(Os::OpenBsd),
            "dragonfly" => Ok(Os::Dragonfly),
            "solaris" => Ok(Os::Solaris),
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
    #[serde(rename = "m68k")]
    M68k,
    #[serde(rename = "csky")]
    Csky,
    #[serde(rename = "mips")]
    Mips,
    #[serde(rename = "mips64")]
    Mips64,
    #[serde(rename = "ppc")]
    PowerPc,
    #[serde(rename = "ppc64")]
    PowerPc64,
    #[serde(rename = "riscv64")]
    RiscV64,
    #[serde(rename = "s390x")]
    S390x,
    #[serde(rename = "sparc")]
    Sparc,
    #[serde(rename = "sparc64")]
    Sparc64,
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
            "m68k" => Ok(Arch::M68k),
            "csky" => Ok(Arch::Csky),
            "mips" => Ok(Arch::Mips),
            "mips64" => Ok(Arch::Mips64),
            "ppc" => Ok(Arch::PowerPc),
            "ppc64" => Ok(Arch::PowerPc64),
            "riscv64" => Ok(Arch::RiscV64),
            "s390x" => Ok(Arch::S390x),
            "sparc" => Ok(Arch::Sparc),
            "sparc64" => Ok(Arch::Sparc64),
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
            "android" => Os::Android,
            "freebsd" => Os::FreeBsd,
            "ios" => Os::Ios,
            "macos" => Os::Mac,
            "netbsd" => Os::NetBsd,
            "openbsd" => Os::NetBsd,
            "dragonfly" => Os::Dragonfly,
            "solaris" => Os::Solaris,
            "windows" => Os::Windows,
            "linux" => match sysinfo::System::distribution_id().as_str() {
                "alpine" => Os::Alpine,
                _ => Os::Linux,
            },
            _ => return None,
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
            "m68k" => Arch::M68k,
            "csky" => Arch::Csky,
            "mips" => Arch::Mips,
            "mips64" => Arch::Mips64,
            "powerpc" => Arch::PowerPc,
            "powerpc64" => Arch::PowerPc64,
            "riscv64" => Arch::RiscV64,
            "s390x" => Arch::S390x,
            "sparc" => Arch::Sparc,
            "sparc64" => Arch::Sparc64,
            _ => return None,
        })
    }
}
