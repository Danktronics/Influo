pub mod project;
pub mod channel;

use serde::{Serialize, Deserialize};

use crate::logger::LogLevel;
use project::Project;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    #[serde(default = "default_update_interval")]
    pub update_interval: u32,
    pub log_level: LogLevel,
    pub default_deploy_path: String,
    pub default_log_path: String,
    pub api: Option<ApiConfiguration>,
    pub projects: Vec<Project>
}

fn default_update_interval() -> u32 {
    30
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfiguration {
    pub http: Option<HttpApiConfiguration>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpApiConfiguration {
    #[serde(default = "default_http_api_port")]
    pub port: u16
}

fn default_http_api_port() -> u16 {
    4200
}

#[cfg(test)]
mod tests {
    use crate::util::filesystem::{read_raw_configuration, read_json_file};
    use anyhow::Error;

    use serde_json::Value;
    
    use crate::model::Configuration;
    
    #[test]
    fn test_configuration_parse() {
        let raw_config: Result<Value, Error> = read_raw_configuration();
        assert!(raw_config.is_ok());
    }

    #[test]
    fn test_configuration_default_stage_order() {
        let config = read_configuration("default_stage_order_test_config");
        assert_eq!(config.projects[0].pipelines[0].stages_order, Some(vec!["test".to_string(), "lint".to_string(), "deploy_staging".to_string(), "deploy_production".to_string(), "cleanup".to_string(), "post".to_string()]))
    }

    fn read_configuration(path: &str) -> Configuration {
        serde_json::from_value(read_json_file(&format!("test_files/{}.json", path)).unwrap()).unwrap()
    }
}
