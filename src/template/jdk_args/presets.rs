use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const AIKARS_FLAGS: &[&str] = &[
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:+PerfDisableSharedMem",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1NewSizePercent=30",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:G1ReservePercent=20",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:MaxGCPauseMillis=200",
    "-XX:MaxTenuringThreshold=1",
    "-XX:SurvivorRatio=32",
    "-Dusing.aikars.flags=https://mcflags.emc.gs",
    "-Daikars.new.flags=true",
];

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum JdkPreset {
    #[serde(rename = "aikars")]
    Aikars,
}

impl JdkPreset {
    pub async fn get_args(&self) -> Vec<String> {
        match self {
            JdkPreset::Aikars => AIKARS_FLAGS.iter().map(|s| s.to_string()).collect(),
        }
    }
}
