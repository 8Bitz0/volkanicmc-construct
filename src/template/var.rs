use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

pub type EnvMap = HashMap<String, String>;
pub type VarMap = HashMap<String, String>;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Var {
    #[serde(rename = "static")]
    Static { name: String, value: String },
    #[serde(rename = "user")]
    User {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VarFormat {
    #[serde(rename = "dollar-curly")]
    DollarCurly,
}

impl VarFormat {
    pub fn formatted(&self, name: impl std::fmt::Display) -> String {
        let fmtd = match self {
            VarFormat::DollarCurly => format!("${{{}}}", name),
        };

        debug!("Formatted variable \"{}\" as \"{}\"", name, fmtd);

        fmtd
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VarProcessError {
    #[error("User dynamic variable not found: {0}")]
    UserVarNotFound(String),
    #[error("Provided user variable does not have name")]
    RawVarWithoutName,
    #[error("Provided user variable does not have value")]
    RawVarWithoutValue,
}

pub fn string_replace(value: String, vars: &VarMap, format: VarFormat) -> String {
    let mut result = value;
    for v in vars {
        let fmtd = format.formatted(v.0);

        debug!("Replacing all instances of \"{}\" with \"{}\"", fmtd, v.1);
        result = result.replace(&fmtd, v.1);
    }
    result
}

pub fn process_vars(
    template_vars: &mut VarMap,
    vars: Vec<Var>,
    env_defs: &EnvMap,
) -> Result<(), VarProcessError> {
    for v in vars {
        match v {
            Var::Static { name, value } => {
                template_vars.insert(name, value);
            }
            Var::User { name, default } => {
                template_vars.insert(name.clone(), {
                    match env_defs.get(&name) {
                        Some(v) => {
                            debug!("Variable {} defined as \"{}\"", name, v);

                            v.to_string()
                        }
                        None => match default {
                            Some(v) => {
                                info!(
                                    "Variable \"{}\" not defined, using default: \"{}\"",
                                    name, v
                                );
                                v.to_string()
                            }
                            None => return Err(VarProcessError::UserVarNotFound(name)),
                        },
                    }
                });
            }
        }
    }

    Ok(())
}
