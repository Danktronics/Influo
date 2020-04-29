use failure::{Error, err_msg};
use serde_json::Value;

pub mod procedure;

use self::procedure::Procedure;

pub struct Project {
    pub url: String,
    pub procedures: Vec<Procedure>,
}

impl Project {
    pub fn new(raw_url: &Value, raw_procedures: &Value) -> Result<Project, Error> {
        if !raw_url.is_string() {
            return Err(err_msg("URL is invalid"));
        }
        let url: &str = raw_url.as_str().unwrap();

        if !raw_procedures.is_array() {
            return Err(err_msg("Procedures is not an array"));
        }
        let raw_procedures_array: &Vec<Value> = raw_procedures.as_array().unwrap();
        let procedures: Vec<Procedure> = Vec::new();
        for raw_procedure in raw_procedures_array {
            procedures.push(Procedure::new(raw_procedure));
        }

        Ok(Project {
            url: url.to_string(),
            procedures: procedures,
        })
    }
}