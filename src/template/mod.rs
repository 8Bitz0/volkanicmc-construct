use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod manage;
pub mod resource;
pub mod var;
pub mod vkinclude;

mod parse;

pub use parse::ParseError;

pub const TEMPLATE_FORMAT: usize = 2;

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
pub struct Template {
    #[serde(rename = "template-format")]
    pub template_format: usize,
    /// Name of the template. The name should briefly describe and identify the template.
    ///
    /// Example: "1.12.2 Vanilla"
    pub name: String,
    /// Longer description of the template.
    ///
    /// Example: "Server running vanilla Minecraft 1.12.2"
    pub description: String,
    /// Simple identifier of the author.
    pub author: Option<String>,
    /// Version of the template.
    pub version: Option<(u64, Option<u64>, Option<u64>)>,
    /// Variables neccessary for the template.
    pub variables: Vec<var::Var>,
    /// Server runtime software.
    pub runtime: resource::ServerRuntimeResource,
    /// List of additional resources (e.g. plugins, mods, configs, etc.)
    pub resources: Vec<resource::GenericResource>,
    /// List of files which should be saved (e.g. worlds, whitelists, etc.)
    pub savables: Vec<PathBuf>,
}

impl Template {
    // TODO: Should use a generic `Path` type
    pub async fn import(file: PathBuf) -> Result<Self, ParseError> {
        parse::file_to_template(file).await
    }
}

impl Default for Template {
    fn default() -> Self {
        Self {
            template_format: TEMPLATE_FORMAT,
            name: "1.20.2 Paper".into(),
            description: "Server running Minecraft 1.20.2 with PaperMC".into(),
            author: Some("Example".into()),
            version: Some((1, Some(0), Some(0))),
            variables: vec![
                var::Var::User {
                    name: "PORT".into(),
                    default: Some("25565".into()),
                },
            ],
            runtime: resource::ServerRuntimeResource::Jdk {
                version: "17".to_string(),
                jar_path: PathBuf::from("server.jar"),
                jdk_args: AIKARS_FLAGS.iter().map(|s| s.to_string()).collect(),
                server_args: vec!["-nogui".to_string()]
            },
            resources: vec![
                resource::GenericResource::Remote {
                    url: "https://api.papermc.io/v2/projects/paper/versions/1.20.2/builds/291/downloads/paper-1.20.2-291.jar".to_string(),
                    user_agent: None,
                    override_name: None,
                    sha512: Some("6179a94b15cbfd141431e509806ab5ce04655effea9866a5a33673b82e7fffe6fb438147565b73c98140e5cf1a5b7d9b083978c46d5239fd08b26863c423a820".to_string()),
                    use_variables: None,
                    archive: None,
                    template_path: PathBuf::from("server.jar"),
                },
                resource::GenericResource::Base64 {
                    base64: "IyBNaW5lY3JhZnQgc2VydmVyIHByb3BlcnRpZXMNCmVuYWJsZS1qbXgtbW9uaXRvcmluZz1mYWxzZQ0KcmNvbi5wb3J0PTI1NTc1DQpsZXZlbC1zZWVkPQ0KZ2FtZW1vZGU9c3Vydml2YWwNCmVuYWJsZS1jb21tYW5kLWJsb2NrPWZhbHNlDQplbmFibGUtcXVlcnk9ZmFsc2UNCmdlbmVyYXRvci1zZXR0aW5ncz17fQ0KZW5mb3JjZS1zZWN1cmUtcHJvZmlsZT1mYWxzZQ0KbGV2ZWwtbmFtZT13b3JsZA0KbW90ZD1BIE1pbmVjcmFmdCBTZXJ2ZXIsIG9uIFZvbGthbmljTUMNCnF1ZXJ5LnBvcnQ9MjU1NjUNCnB2cD10cnVlDQpnZW5lcmF0ZS1zdHJ1Y3R1cmVzPXRydWUNCm1heC1jaGFpbmVkLW5laWdoYm9yLXVwZGF0ZXM9MTAwMDAwMA0KZGlmZmljdWx0eT1ub3JtYWwNCm5ldHdvcmstY29tcHJlc3Npb24tdGhyZXNob2xkPTI1Ng0KbWF4LXRpY2stdGltZT02MDAwMA0KcmVxdWlyZS1yZXNvdXJjZS1wYWNrPWZhbHNlDQp1c2UtbmF0aXZlLXRyYW5zcG9ydD10cnVlDQptYXgtcGxheWVycz04DQpvbmxpbmUtbW9kZT10cnVlDQplbmFibGUtc3RhdHVzPXRydWUNCmFsbG93LWZsaWdodD1mYWxzZQ0KaW5pdGlhbC1kaXNhYmxlZC1wYWNrcz0NCmJyb2FkY2FzdC1yY29uLXRvLW9wcz10cnVlDQp2aWV3LWRpc3RhbmNlPTgNCnNlcnZlci1pcD0NCnJlc291cmNlLXBhY2stcHJvbXB0PQ0KYWxsb3ctbmV0aGVyPXRydWUNCnNlcnZlci1wb3J0PSR7UE9SVH0NCmVuYWJsZS1yY29uPWZhbHNlDQpzeW5jLWNodW5rLXdyaXRlcz10cnVlDQpvcC1wZXJtaXNzaW9uLWxldmVsPTQNCnByZXZlbnQtcHJveHktY29ubmVjdGlvbnM9ZmFsc2UNCmhpZGUtb25saW5lLXBsYXllcnM9ZmFsc2UNCnJlc291cmNlLXBhY2s9DQplbnRpdHktYnJvYWRjYXN0LXJhbmdlLXBlcmNlbnRhZ2U9MTAwDQpzaW11bGF0aW9uLWRpc3RhbmNlPTEwDQpyY29uLnBhc3N3b3JkPQ0KcGxheWVyLWlkbGUtdGltZW91dD0wDQpmb3JjZS1nYW1lbW9kZT1mYWxzZQ0KcmF0ZS1saW1pdD0wDQpoYXJkY29yZT1mYWxzZQ0Kd2hpdGUtbGlzdD1mYWxzZQ0KYnJvYWRjYXN0LWNvbnNvbGUtdG8tb3BzPXRydWUNCnNwYXduLW5wY3M9dHJ1ZQ0Kc3Bhd24tYW5pbWFscz10cnVlDQpsb2ctaXBzPXRydWUNCmZ1bmN0aW9uLXBlcm1pc3Npb24tbGV2ZWw9Mg0KaW5pdGlhbC1lbmFibGVkLXBhY2tzPXZhbmlsbGENCmxldmVsLXR5cGU9bWluZWNyYWZ0XDpub3JtYWwNCnRleHQtZmlsdGVyaW5nLWNvbmZpZz0NCnNwYXduLW1vbnN0ZXJzPXRydWUNCmVuZm9yY2Utd2hpdGVsaXN0PWZhbHNlDQpzcGF3bi1wcm90ZWN0aW9uPTE2DQpyZXNvdXJjZS1wYWNrLXNoYTE9DQptYXgtd29ybGQtc2l6ZT0yOTk5OTk4NA==".into(),
                    use_variables: Some(var::VarFormat::DollarCurly),
                    template_path: "server.properties".into(),
                },
            ],
            savables: vec![
                PathBuf::from("logs/"),
                PathBuf::from("world/"),
                PathBuf::from("world_nether/"),
                PathBuf::from("world_the_end/"),
                PathBuf::from(".console_history"),
                PathBuf::from("banned-ips.json"),
                PathBuf::from("banned-players.json"),
                PathBuf::from("ops.json"),
                PathBuf::from("permissions.yml"),
                PathBuf::from("usercache.json"),
                PathBuf::from("version_history.json"),
                PathBuf::from("whitelist.json"),
            ]
        }
    }
}
