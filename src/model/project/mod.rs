use anyhow::{Error, anyhow};
use serde_json::Value;
use serde::Serialize;

pub mod procedure;
pub mod branch;

use self::{
    procedure::Procedure,
    branch::Branch
};

#[derive(Debug, Serialize)]
pub struct Project {
    pub url: String,
    pub procedures: Vec<Procedure>,
    pub branches: Vec<Branch>,
}

impl Project {
    pub fn new(raw_project: &Value, raw_default_deploy_path: Option<&Value>) -> Result<Project, Error> {
        if !raw_project["url"].is_string() {
            return Err(anyhow!("URL is invalid"));
        }
        let url: &str = raw_project["url"].as_str().unwrap();

        if !raw_project["procedures"].is_array() {
            return Err(anyhow!("Procedures is invalid"));
        }
        let raw_procedures_array: &Vec<Value> = raw_project["procedures"].as_array().unwrap();
        let mut procedures: Vec<Procedure> = Vec::new();
        for raw_procedure in raw_procedures_array {
            procedures.push(Procedure::new(raw_procedure, raw_default_deploy_path)?);
        }

        Ok(Project {
            url: url.to_string(),
            procedures,
            branches: Vec::new(),
        })
    }

    pub fn update_branches(&mut self, branches: Vec<Branch>) {
        self.branches = branches;
    }
}
