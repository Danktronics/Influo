use failure::{Error, err_msg};

pub struct Procedure {
    pub name: String,
    pub commands: Vec<String>,
    pub environment: String,
    pub condition: String,
    pub branches: Vec<String>,
}

impl Procedure {
    pub fn new(data: Vec<Value>) -> Result<Procedure, Error> {
        let raw_name: Value = data["name"];
        if !raw_name.is_string() {
            return err_msg("Name is invalid in procedure");
        }
        let name: &String = raw_name.as_str().unwrap();

        let raw_commands: Value = data["commands"];
        if !raw_commands.is_array() {
            return err_msg("Commands is invalid in procedure");
        }
        let raw_commands_array: &Vec<String> = raw_commands.as_array().unwrap();
        let commands: Vec<String> = Vec::new();
        for raw_command in raw_commands_array {
            if !raw_command.is_string() {
                return err_msg("Procedure command is invalid");
            }
            commands.push(raw_command.as_str().unwrap());
        }

        let raw_environment: Value = data["environment"];
        if !raw_environment.is_string() {
            return err_msg("Environment is invalid in procedure");
        }
        let environment: &String = raw_environment.as_str().unwrap();

        let raw_condition: Value = data["condition"];
        if !raw_condition.is_string() {
            return err_msg("Condition is invalid in procedure");
        }
        let condition: &String = raw_condition.as_str().unwrap();

        let raw_branches: Value = data["branches"];
        if !raw_branches.is_array() {
            return err_msg("Branches is invalid in procedure");
        }
        let raw_branches_array: &Vec<String> = raw_branches.as_array().unwrap();
        let branches: Vec<String> = Vec::new();
        for raw_branch in raw_branches_array {
            if !raw_branch.is_string() {
                return err_msg("Procedure branch is invalid");
            }
            branches.push(raw_branch.as_str().unwrap());
        }
    }
}