use failure::{Error, err_msg};
use serde_json::Value;

#[derive(Debug)]
pub struct Procedure {
    pub name: String,
    pub commands: Vec<String>,
    pub environment: String,
    pub condition: String,
    pub branches: Vec<String>,
}

impl Procedure {
    pub fn new(data: &Value) -> Result<Procedure, Error> {
        let raw_name: &Value = &data["name"];
        if !raw_name.is_string() {
            return Err(err_msg("Name is invalid in procedure"));
        }
        let name: &str = raw_name.as_str().unwrap();

        let raw_commands: &Value = &data["commands"];
        if !raw_commands.is_array() {
            return Err(err_msg("Commands is invalid in procedure"));
        }
        let raw_commands_array: &Vec<Value> = raw_commands.as_array().unwrap();
        let mut commands: Vec<String> = Vec::new();
        for raw_command in raw_commands_array {
            if !raw_command.is_string() {
                return Err(err_msg("Procedure command is invalid"));
            }
            commands.push(raw_command.as_str().unwrap().to_string());
        }

        let raw_environment: &Value = &data["environment"];
        if !raw_environment.is_string() {
            return Err(err_msg("Environment is invalid in procedure"));
        }
        let environment: &str = raw_environment.as_str().unwrap();

        let raw_condition: &Value = &data["condition"];
        if !raw_condition.is_string() {
            return Err(err_msg("Condition is invalid in procedure"));
        }
        let condition: &str = raw_condition.as_str().unwrap();

        let raw_branches: &Value = &data["branches"];
        if !raw_branches.is_array() {
            return Err(err_msg("Branches is invalid in procedure"));
        }
        let raw_branches_array: &Vec<Value> = raw_branches.as_array().unwrap();
        let mut branches: Vec<String> = Vec::new();
        for raw_branch in raw_branches_array {
            if !raw_branch.is_string() {
                return Err(err_msg("Procedure branch is invalid"));
            }
            branches.push(raw_branch.as_str().unwrap().to_string());
        }

        Ok(Procedure {
            name: name.to_string(),
            commands: commands,
            environment: environment.to_string(),
            condition: condition.to_string(),
            branches: branches,
        })
    }
}