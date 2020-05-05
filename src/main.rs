// Dependencies
use std::fs;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use failure::{Error, err_msg};
use serde_json::Value;

// Project Modules
#[macro_use]
mod logger;
mod model;
mod system_cmd;
mod procedure_manager;
mod webserver;

use model::project::Project;
use model::project::branch::Branch;
use model::channel::{ThreadConnection, ThreadProcedureConnection};
use system_cmd::get_remote_git_repository_commits;
use procedure_manager::run_project_procedures;
//use webserver::start_webserver;
use logger::{LOGGER, Logger};

fn main() -> Result<(), Error> {
    info!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;
    if config["log_level"].is_string() {
        LOGGER.lock().unwrap().set_log_level(Logger::string_to_log_level(&config["log_level"].as_str().unwrap()));
    }

    // Process and cache projects
    let raw_projects: &Value = &config["projects"];
    if !raw_projects.is_array() {
        return Err(err_msg("Projects is invalid"));
    }
    let raw_projects_array: &Vec<Value> = raw_projects.as_array().unwrap();
    let projects: Arc<Mutex<Vec<Project>>> = Arc::new(Mutex::new(Vec::new()));
    for raw_project in raw_projects_array {
        let mut temp_projects = projects.lock().unwrap();
        temp_projects.push(Project::new(&raw_project, &config["default_deploy_path"])?);
    }

    // Retrieve update interval and start the updater thread
    let raw_update_interval: &Value = &config["update_interval"];
    let update_interval: u32 = if raw_update_interval.is_null() || !raw_update_interval.is_number() {
        30
    } else {
        let interval: Option<u64> = raw_update_interval.as_u64();
        if interval.is_none() || interval.unwrap() > u32::MAX as u64 {
            panic!("The integer provided exceeded the u32 max");
        }
        interval.unwrap() as u32 * 1000
    };
    let updater_communication: ThreadConnection = ThreadConnection::new();
    let thread_join_handle: thread::JoinHandle<()> = setup_updater_thread(update_interval, projects, updater_communication);

    
    // Start webserver (For API)
    let raw_port: &Value = &config["port"];
    let port: u16 = if !raw_port.is_number() {
        9050
    } else {
        let port: Option<u64> = raw_port.as_u64();
        if port.is_none() || port.unwrap() > u16::MAX as u64 {
            panic!("Invalid webserver port");
        }
        port.unwrap() as u16
    };
    //start_webserver(port);

    Ok(())
}

/// Spawns the updater thread for checking updates and controlling procedures
/// Interval should be in milliseconds
fn setup_updater_thread(interval: u32, projects: Arc<Mutex<Vec<Project>>>, main_communication: ThreadConnection) -> thread::JoinHandle<()> {
    info!("Spawning updater thread");

    let procedure_thread_connections: Vec<ThreadProcedureConnection> = Vec::new();

    let updater_projects_ref = Arc::clone(&projects);
    thread::spawn(move || {
        let mut unlocked_projects = updater_projects_ref.lock().unwrap();
        loop {
            debug!(format!("Updater thread sleeping for {} seconds", interval));
            thread::sleep(Duration::from_millis(interval as u64));
            debug!("Checking project repositories for updates");
            for project in &mut *unlocked_projects {
                let query_result = get_remote_git_repository_commits(&project.url);
                if query_result.is_err() {
                    error!(format!("Failed to query commits for project with url {} and error:\n{}", project.url, query_result.err().unwrap()));
                    continue;
                }

                let branches = query_result.unwrap();
                for branch in &branches {
                    let short_hash = branch.latest_commit_hash.chars().take(5);
                    debug!(format!("Current branch is {}. Current short commit hash is {}", branch.name, short_hash));
                    let branch_search = project.branches.iter().find(|&b| b.name == branch.name);
                    if branch_search.is_some() && branch_search.unwrap().latest_commit_hash == branch.latest_commit_hash {
                        continue;
                    }

                    info!(format!("Updating to commit {} in the {} branch...", short_hash, branch.name));
                    let procedure_immediate_result = run_project_procedures(&project, &branch, procedure_thread_connections);

                    if procedure_immediate_result.is_err() {
                        error!(format!("Error occurred while running procedure: {:?}", procedure_immediate_result));
                    } else {
                        info!("Update most likely succeeded"); // Horribly incorrect
                    }
                }

                project.update_branches(branches);
            }
        }
    })
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}
