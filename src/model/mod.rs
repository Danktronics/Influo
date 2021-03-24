pub mod project;
pub mod channel;

use serde::{Serialize, Deserialize};

use crate::logger::LogLevel;
use project::Project;

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    #[serde(default = "default_update_interval")]
    pub update_interval: u32,
    pub log_level: LogLevel,
    pub default_deploy_path: String,
    pub api: Option<ApiConfiguration>,
    pub projects: Vec<Project>
}

fn default_update_interval() -> u32 {
    30
}

#[derive(Serialize, Deserialize)]
pub struct ApiConfiguration {
    pub http: Option<HttpApiConfiguration>
}

#[derive(Serialize, Deserialize)]
pub struct HttpApiConfiguration {
    #[serde(default = "default_http_api_port")]
    pub port: u16
}

fn default_http_api_port() -> u16 {
    4200
}
