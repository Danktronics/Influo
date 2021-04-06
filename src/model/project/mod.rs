use serde::{Serialize, Deserialize};

pub mod procedure;
pub mod branch;

use self::{
    procedure::Procedure,
    branch::Branch
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub url: String,
    pub procedures: Vec<Procedure>,
    #[serde(skip)]
    pub branches: Vec<Branch>,
    #[serde(default)]
    pub persistent: bool
}

impl Project {
    pub fn update_branches(&mut self, branches: Vec<Branch>) {
        self.branches = branches;
    }
}
