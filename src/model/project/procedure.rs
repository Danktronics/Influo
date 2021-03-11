use anyhow::{Error, anyhow};
use serde_json::Value;

#[derive(Debug)]
pub struct Procedure {
    pub name: String,
    pub commands: Vec<String>,
    pub environment: String,
    pub condition: String,
    pub deploy_path: String,
    pub auto_restart: AutoRestartPolicy,
    pub branches: Vec<String>,
    pub log: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AutoRestartPolicy {
    Always, // If the command was unsuccessful, restart
    Never, // If the command was unsuccessful, don't restart
    ExclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is NOT one of the exclusion codes restart
    InclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is one of the inclusion codes restart
}

impl Procedure {
    pub fn new(raw_procedure: &Value, raw_default_deploy_path: Option<&Value>) -> Result<Procedure, Error> {
        let name: &str = match raw_procedure.get("name") {
            Some(raw_name) => match raw_name.as_str() {
                Some(s) => s,
                None => return Err(anyhow!("Name is invalid in procedure")),
            },
            None => return Err(anyhow!("Name not found in procedure")),
        };

        let raw_commands: &Vec<Value> = match raw_procedure.get("commands") {
            Some(v) => match v.as_array() {
                Some(v) => v,
                None => return Err(anyhow!("Commands is invalid in procedure")),
            },
            None => return Err(anyhow!("Commands not found in procedure")),
        };
        let mut commands: Vec<String> = Vec::new();
        for raw_command in raw_commands {
            match raw_command.as_str() {
                Some(s) => commands.push(s.to_string()),
                None => return Err(anyhow!("Procedure command is invalid")),
            }
        }

        let environment: &str = match raw_procedure.get("environment") {
            Some(raw_environment) => match raw_environment.as_str() {
                Some(s) => s,
                None => return Err(anyhow!("Environment is invalid in procedure")),
            },
            None => return Err(anyhow!("Environment not found in procedure"))
        };

        let condition: &str = match raw_procedure.get("condition") {
            Some(raw_condition) => match raw_condition.as_str() {
                Some(s) => s,
                None => return Err(anyhow!("Condition is invalid in procedure")),
            },
            None => return Err(anyhow!("Condition not found in procedure")),
        };

        let deploy_path: &str = match raw_procedure.get("deploy_path") {
            Some(raw_deploy_path) => match raw_deploy_path.as_str() {
                Some(s) => s,
                None => return Err(anyhow!("Deploy path is invalid in procedure")),
            },
            None => match raw_default_deploy_path {
                Some(raw_default_deploy_path) => match raw_default_deploy_path.as_str() {
                    Some(s) => s,
                    None => return Err(anyhow!("Default deploy path is invalid")),
                },
                None => return Err(anyhow!("Both default and procedure deploy paths were not set"))
            }
        };

        let auto_restart: AutoRestartPolicy = match raw_procedure.get("auto_restart") {
            Some(raw_auto_restart) => match raw_auto_restart.as_bool() {
                Some(raw_auto_restart_bool) => {
                    if raw_auto_restart_bool {
                        AutoRestartPolicy::Always
                    } else {
                        AutoRestartPolicy::Never
                    }
                },
                None => match raw_auto_restart.as_object() {
                    Some(raw_auto_restart_object) => {
                        if raw_auto_restart_object.contains_key("only") {
                            match raw_auto_restart_object.get("only").unwrap().as_array() {
                                Some(raw_auto_restart_inclusion_codes) => {
                                    let mut inclusion_codes: Vec<i32> = Vec::new();
                                    for raw_code in raw_auto_restart_inclusion_codes {
                                        match raw_code.as_u64() {
                                            Some(raw_code_u64) => {
                                                if raw_code_u64 > std::i32::MAX as u64 {
                                                    return Err(anyhow!("An auto restart integer provided exceeded the i32 max"));
                                                }

                                                inclusion_codes.push(raw_code_u64 as i32);
                                            },
                                            None => return Err(anyhow!("An auto restart value is not a valid i32"))
                                        }
                                    }
                                    AutoRestartPolicy::InclusionCodes(inclusion_codes)
                                },
                                None => return Err(anyhow!("Auto restart rule inclusion codes not an array"))
                            }
                        } else if raw_auto_restart_object.contains_key("not") {
                            match raw_auto_restart_object.get("not").unwrap().as_array() {
                                Some(raw_auto_restart_inclusion_codes) => {
                                    let mut exclusion_codes: Vec<i32> = Vec::new();
                                    for raw_code in raw_auto_restart_inclusion_codes {
                                        match raw_code.as_u64() {
                                            Some(raw_code_u64) => {
                                                if raw_code_u64 > std::i32::MAX as u64 {
                                                    return Err(anyhow!("An auto restart integer provided exceeded the i32 max"));
                                                }

                                                exclusion_codes.push(raw_code_u64 as i32);
                                            },
                                            None => return Err(anyhow!("An auto restart value is not a valid i32"))
                                        }
                                    }
                                    AutoRestartPolicy::ExclusionCodes(exclusion_codes)
                                },
                                None => return Err(anyhow!("Auto restart rule exclusion codes not an array"))
                            }
                        } else {
                            return Err(anyhow!("Auto restart rule object does not specify a valid rule"));
                        }
                    },
                    None => return Err(anyhow!("Auto restart rule is not an object or boolean"))
                }
            },
            None => AutoRestartPolicy::Never
        };

        let raw_branches: &Vec<Value> = match raw_procedure.get("branches") {
            Some(v) => match v.as_array() {
                Some(v) => v,
                None => return Err(anyhow!("Branches is invalid in procedure")),
            },
            None => return Err(anyhow!("Branches not found in procedure")),
        };
        let mut branches: Vec<String> = Vec::new();
        for raw_branch in raw_branches {
            match raw_branch.as_str() {
                Some(b) => branches.push(b.to_string()),
                None => return Err(anyhow!("Procedure branch is invalid")),
            }
        }

        let log: Option<String> = match raw_procedure.get("log") {
            Some(v) => {
                match v.as_str() {
                    Some(s) => Some(s.to_string()),
                    None => return Err(anyhow!("Log format is invalid in procedure")),
                }
            },
            None => None
        };

        Ok(Procedure {
            name: name.to_string(),
            commands,
            environment: environment.to_string(),
            condition: condition.to_string(),
            deploy_path: deploy_path.to_string(),
            auto_restart,
            branches,
            log,
        })
    }
}
