use anyhow::{Error, anyhow};
use serde_json::Value;
use serde::{Serialize, Deserialize, Deserializer};

pub mod procedure;
pub mod branch;

use self::{
    procedure::Procedure,
    branch::Branch
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub url: String,
    pub procedures: Vec<Procedure>,
    pub branches: Vec<Branch>,
}

impl Project {
    pub fn update_branches(&mut self, branches: Vec<Branch>) {
        self.branches = branches;
    }
}
