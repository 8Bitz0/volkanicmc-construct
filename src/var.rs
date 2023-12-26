use serde::{Deserialize, Serialize};

pub trait Var {
    fn format(&self) -> VarFormat;
    fn formatted(&self) -> String;
    fn name(&self) -> String;
    fn value(&self) -> String;
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VarFormat {
    DollarDoubleCurly,
}

impl VarFormat {
    fn formatted(&self, name: impl std::fmt::Display) -> String {
        match self {
            VarFormat::DollarDoubleCurly => format!("${{{}}}", name),
        }
    }
}

impl Default for VarFormat {
    fn default() -> Self {
        Self::DollarDoubleCurly
    }
}

pub struct StaticVar {
    format: VarFormat,
    name: String,
    value: String,
}

impl Var for StaticVar {
    fn format(&self) -> VarFormat { self.format.clone() }
    fn formatted(&self) -> String { self.format.formatted(&self.name) }
    fn name(&self) -> String { self.name.clone() }
    fn value(&self) -> String { self.value.clone() }
}

pub fn string_replace(template: String, vars: Vec<Vars>) -> String {
    let mut result = template;
    for var in vars {
        let var = match var {
            Vars::Static { format, name, value } => StaticVar { format, name, value },
        };
        result = result.replace(&var.formatted(), &var.value());
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Vars {
    #[serde(rename = "static")]
    Static { format: VarFormat, name: String, value: String },
}
