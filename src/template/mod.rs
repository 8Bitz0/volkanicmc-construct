use serde::{Deserialize, Serialize};
use std::path;

use crate::var;

pub mod manage;
mod parse;
pub mod resource;
pub mod vkinclude;

pub use parse::ParseError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Template {
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
    /// List of template variables.
    pub variables: Vec<var::Vars>,
    /// Server runtime software.
    pub runtime: resource::ServerRuntimeResource,
    /// Server software resource.
    pub server: resource::ServerExecResource,
    /// List of additional resources (e.g. plugins, mods, configs, etc.)
    pub resources: Vec<resource::GenericResource>,
}

impl Template {
    pub async fn import(file: path::PathBuf) -> Result<Self, ParseError> {
        parse::json_to_template(file).await
    }
}

impl Default for Template {
    fn default() -> Self {
        Self {
            name: "1.20.2 Paper".into(),
            description: "Server running Minecraft 1.20.2 with PaperMC".into(),
            author: Some("Example".into()),
            version: Some((1, Some(0), Some(0))),
            variables: vec![],
            runtime: resource::ServerRuntimeResource::Jdk { version: "17".to_string() },
            server: resource::ServerExecResource::Java {
                url: "https://api.papermc.io/v2/projects/paper/versions/1.20.2/builds/291/downloads/paper-1.20.2-291.jar".into(),
                sha512: "6179a94b15cbfd141431e509806ab5ce04655effea9866a5a33673b82e7fffe6fb438147565b73c98140e5cf1a5b7d9b083978c46d5239fd08b26863c423a820".into(),
                args: "-nogui".into(),
            },
            resources: vec![
                resource::GenericResource::Base64 {
                    base64: "ZW5hYmxlLWpteC1tb25pdG9yaW5nPWZhbHNlCnJjb24ucG9ydD0yNTU3NQpsZXZlbC1zZWVkPQpnYW1lbW9kZT1zdXJ2aXZhbAplbmFibGUtY29tbWFuZC1ibG9jaz1mYWxzZQplbmFibGUtcXVlcnk9ZmFsc2UKZ2VuZXJhdG9yLXNldHRpbmdzPXt9CmVuZm9yY2Utc2VjdXJlLXByb2ZpbGU9dHJ1ZQpsZXZlbC1uYW1lPXdvcmxkCm1vdGQ9QSBNaW5lY3JhZnQgU2VydmVyCnF1ZXJ5LnBvcnQ9MjU1NjUKcHZwPXRydWUKZ2VuZXJhdGUtc3RydWN0dXJlcz10cnVlCm1heC1jaGFpbmVkLW5laWdoYm9yLXVwZGF0ZXM9MTAwMDAwMApkaWZmaWN1bHR5PWVhc3kKbmV0d29yay1jb21wcmVzc2lvbi10aHJlc2hvbGQ9MjU2Cm1heC10aWNrLXRpbWU9NjAwMDAKcmVxdWlyZS1yZXNvdXJjZS1wYWNrPWZhbHNlCnVzZS1uYXRpdmUtdHJhbnNwb3J0PXRydWUKbWF4LXBsYXllcnM9MjAKb25saW5lLW1vZGU9dHJ1ZQplbmFibGUtc3RhdHVzPXRydWUKYWxsb3ctZmxpZ2h0PWZhbHNlCmluaXRpYWwtZGlzYWJsZWQtcGFja3M9CmJyb2FkY2FzdC1yY29uLXRvLW9wcz10cnVlCnZpZXctZGlzdGFuY2U9MTAKc2VydmVyLWlwPQpyZXNvdXJjZS1wYWNrLXByb21wdD0KYWxsb3ctbmV0aGVyPXRydWUKc2VydmVyLXBvcnQ9MjU1NjUKZW5hYmxlLXJjb249ZmFsc2UKc3luYy1jaHVuay13cml0ZXM9dHJ1ZQpvcC1wZXJtaXNzaW9uLWxldmVsPTQKcHJldmVudC1wcm94eS1jb25uZWN0aW9ucz1mYWxzZQpoaWRlLW9ubGluZS1wbGF5ZXJzPWZhbHNlCnJlc291cmNlLXBhY2s9CmVudGl0eS1icm9hZGNhc3QtcmFuZ2UtcGVyY2VudGFnZT0xMDAKc2ltdWxhdGlvbi1kaXN0YW5jZT0xMApyY29uLnBhc3N3b3JkPQpwbGF5ZXItaWRsZS10aW1lb3V0PTAKZm9yY2UtZ2FtZW1vZGU9ZmFsc2UKcmF0ZS1saW1pdD0wCmhhcmRjb3JlPWZhbHNlCndoaXRlLWxpc3Q9ZmFsc2UKYnJvYWRjYXN0LWNvbnNvbGUtdG8tb3BzPXRydWUKc3Bhd24tbnBjcz10cnVlCnNwYXduLWFuaW1hbHM9dHJ1ZQpsb2ctaXBzPXRydWUKZnVuY3Rpb24tcGVybWlzc2lvbi1sZXZlbD0yCmluaXRpYWwtZW5hYmxlZC1wYWNrcz12YW5pbGxhCmxldmVsLXR5cGU9bWluZWNyYWZ0XDpub3JtYWwKdGV4dC1maWx0ZXJpbmctY29uZmlnPQpzcGF3bi1tb25zdGVycz10cnVlCmVuZm9yY2Utd2hpdGVsaXN0PWZhbHNlCnNwYXduLXByb3RlY3Rpb249MTYKcmVzb3VyY2UtcGFjay1zaGExPQptYXgtd29ybGQtc2l6ZT0yOTk5OTk4NA==".into(),
                    variable_substitution: false,
                    template_path: "server.properties".into(),
                }
            ],
        }
    }
}
