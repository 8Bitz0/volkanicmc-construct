use serde::{Deserialize, Serialize};
use std::path;

pub mod jdk_args;
pub mod manage;
pub mod resource;
pub mod var;
pub mod vkinclude;

mod parse;

pub use parse::ParseError;

use crate::persistence::PersistentObject;

pub const TEMPLATE_FORMAT: usize = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Server software resource.
    pub server: resource::ServerExecResource,
    /// List of additional resources (e.g. plugins, mods, configs, etc.)
    pub resources: Vec<resource::GenericResource>,
    /// Persistent objects
    #[serde(rename = "persistent-objects")]
    pub persistent_objects: Vec<PersistentObject>,
}

impl Template {
    pub async fn import(file: path::PathBuf) -> Result<Self, ParseError> {
        parse::file_to_template(file).await
    }
}

impl Default for Template {
    fn default() -> Self {
        Self {
            template_format: TEMPLATE_FORMAT,
            name: "1.20.4 Paper".into(),
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
                additional_args: Some(jdk_args::JdkArguments::Preset(
                    jdk_args::presets::JdkPreset::Aikars
                ))
            },
            server: resource::ServerExecResource::Java {
                url: "https://api.papermc.io/v2/projects/paper/versions/1.20.2/builds/291/downloads/paper-1.20.2-291.jar".into(),
                sha512: "6179a94b15cbfd141431e509806ab5ce04655effea9866a5a33673b82e7fffe6fb438147565b73c98140e5cf1a5b7d9b083978c46d5239fd08b26863c423a820".into(),
                args: "-nogui".into(),
            },
            resources: vec![
                resource::GenericResource::Base64 {
                    base64: "IyBNaW5lY3JhZnQgc2VydmVyIHByb3BlcnRpZXMNCmVuYWJsZS1qbXgtbW9uaXRvcmluZz1mYWxzZQ0KcmNvbi5wb3J0PTI1NTc1DQpsZXZlbC1zZWVkPQ0KZ2FtZW1vZGU9c3Vydml2YWwNCmVuYWJsZS1jb21tYW5kLWJsb2NrPWZhbHNlDQplbmFibGUtcXVlcnk9ZmFsc2UNCmdlbmVyYXRvci1zZXR0aW5ncz17fQ0KZW5mb3JjZS1zZWN1cmUtcHJvZmlsZT1mYWxzZQ0KbGV2ZWwtbmFtZT13b3JsZA0KbW90ZD1BIE1pbmVjcmFmdCBTZXJ2ZXIsIG9uIFZvbGthbmljTUMNCnF1ZXJ5LnBvcnQ9MjU1NjUNCnB2cD10cnVlDQpnZW5lcmF0ZS1zdHJ1Y3R1cmVzPXRydWUNCm1heC1jaGFpbmVkLW5laWdoYm9yLXVwZGF0ZXM9MTAwMDAwMA0KZGlmZmljdWx0eT1ub3JtYWwNCm5ldHdvcmstY29tcHJlc3Npb24tdGhyZXNob2xkPTI1Ng0KbWF4LXRpY2stdGltZT02MDAwMA0KcmVxdWlyZS1yZXNvdXJjZS1wYWNrPWZhbHNlDQp1c2UtbmF0aXZlLXRyYW5zcG9ydD10cnVlDQptYXgtcGxheWVycz04DQpvbmxpbmUtbW9kZT10cnVlDQplbmFibGUtc3RhdHVzPXRydWUNCmFsbG93LWZsaWdodD1mYWxzZQ0KaW5pdGlhbC1kaXNhYmxlZC1wYWNrcz0NCmJyb2FkY2FzdC1yY29uLXRvLW9wcz10cnVlDQp2aWV3LWRpc3RhbmNlPTgNCnNlcnZlci1pcD0NCnJlc291cmNlLXBhY2stcHJvbXB0PQ0KYWxsb3ctbmV0aGVyPXRydWUNCnNlcnZlci1wb3J0PSR7UE9SVH0NCmVuYWJsZS1yY29uPWZhbHNlDQpzeW5jLWNodW5rLXdyaXRlcz10cnVlDQpvcC1wZXJtaXNzaW9uLWxldmVsPTQNCnByZXZlbnQtcHJveHktY29ubmVjdGlvbnM9ZmFsc2UNCmhpZGUtb25saW5lLXBsYXllcnM9ZmFsc2UNCnJlc291cmNlLXBhY2s9DQplbnRpdHktYnJvYWRjYXN0LXJhbmdlLXBlcmNlbnRhZ2U9MTAwDQpzaW11bGF0aW9uLWRpc3RhbmNlPTEwDQpyY29uLnBhc3N3b3JkPQ0KcGxheWVyLWlkbGUtdGltZW91dD0wDQpmb3JjZS1nYW1lbW9kZT1mYWxzZQ0KcmF0ZS1saW1pdD0wDQpoYXJkY29yZT1mYWxzZQ0Kd2hpdGUtbGlzdD1mYWxzZQ0KYnJvYWRjYXN0LWNvbnNvbGUtdG8tb3BzPXRydWUNCnNwYXduLW5wY3M9dHJ1ZQ0Kc3Bhd24tYW5pbWFscz10cnVlDQpsb2ctaXBzPXRydWUNCmZ1bmN0aW9uLXBlcm1pc3Npb24tbGV2ZWw9Mg0KaW5pdGlhbC1lbmFibGVkLXBhY2tzPXZhbmlsbGENCmxldmVsLXR5cGU9bWluZWNyYWZ0XDpub3JtYWwNCnRleHQtZmlsdGVyaW5nLWNvbmZpZz0NCnNwYXduLW1vbnN0ZXJzPXRydWUNCmVuZm9yY2Utd2hpdGVsaXN0PWZhbHNlDQpzcGF3bi1wcm90ZWN0aW9uPTE2DQpyZXNvdXJjZS1wYWNrLXNoYTE9DQptYXgtd29ybGQtc2l6ZT0yOTk5OTk4NA==".into(),
                    use_variables: Some(var::VarFormat::DollarCurly),
                    template_path: "server.properties".into(),
                },
            ],
            persistent_objects: vec![
                PersistentObject::Directory(path::PathBuf::from("world/")),
                PersistentObject::Directory(path::PathBuf::from("world_nether/")),
                PersistentObject::Directory(path::PathBuf::from("world_the_end/")),
                PersistentObject::File(path::PathBuf::from(".console_history")),
                PersistentObject::File(path::PathBuf::from("banned-ips.json")),
                PersistentObject::File(path::PathBuf::from("banned-players.json")),
                PersistentObject::File(path::PathBuf::from("ops.json")),
                PersistentObject::File(path::PathBuf::from("whitelist.json")),
                PersistentObject::File(path::PathBuf::from("version_history.json")),
                PersistentObject::File(path::PathBuf::from("usercache.json")),
            ]
        }
    }
}
