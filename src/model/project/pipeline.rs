use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use super::procedure::Procedure;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pipeline {
    pub name: String,
    pub stages: Option<Vec<String>>,
    pub branches: Vec<String>,
    pub deploy_path: Option<String>,
    pub log: Option<Log>,
    pub condition: Condition,
    pub procedures: HashMap<String, Vec<Procedure>>,
    #[serde(default)]
    pub persistent: bool
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Log {
    pub template: Option<String>,
    pub console: Option<bool>,
    pub save_to_file: Option<bool>,
    pub file_path: Option<String>,
    pub in_memory: Option<bool>
}

impl Log {
    pub fn is_enabled(&self) -> bool {
        self.console.unwrap_or(false) || self.save_to_file.unwrap_or(false) || self.in_memory.unwrap_or(false)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    Automatic,
    Manual
}
