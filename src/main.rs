// Dependencies
use std::{
    fs,
    thread,
    time::Duration,
    sync::{Arc, Mutex, RwLock}
};
use failure::{Error, err_msg};
use serde_json::Value;

// Project Modules
#[macro_use]
mod logger;
mod model;
mod system_cmd;
mod procedure_manager;

use model::{
    project::Project,
    channel::message::Command,
    channel::ThreadProcedureConnection
};
use system_cmd::get_remote_git_repository_commits;
use procedure_manager::run_project_procedure;
use logger::{LOGGER, Logger};

fn main() -> Result<(), Error> {
    info!("Influo is running!");

    // Load Configuration
    let raw_config: Result<Value, Error> = read_configuration();
    if raw_config.is_err() {
        error!("Configuration not found");
        return Err(raw_config.err().unwrap());
    }
    let config: Value = raw_config.unwrap();
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
    // let updater_communication: ThreadConnection = ThreadConnection::new();
    let thread_join_handle: thread::JoinHandle<()> = setup_updater_thread(update_interval, projects);
    thread_join_handle.join().unwrap();

    Ok(())
}

/// Spawns the updater thread for checking updates and controlling procedures
/// Interval should be in milliseconds
fn setup_updater_thread(interval: u32, projects: Arc<Mutex<Vec<Project>>>) -> thread::JoinHandle<()> {
    info!("Spawning updater thread");

    let mut procedure_thread_connections: Vec<Arc<RwLock<ThreadProcedureConnection>>> = Vec::new();

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
                    let short_hash: String = branch.latest_commit_hash.chars().take(5).collect();
                    debug!(format!("Current branch is {}. Current short commit hash is {}", branch.name, short_hash));
                    let branch_search = project.branches.iter().find(|&b| b.name == branch.name);
                    if branch_search.is_some() && branch_search.unwrap().latest_commit_hash == branch.latest_commit_hash {
                        continue;
                    }

                    info!(format!("Updating to commit {} in the {} branch...", short_hash, branch.name));
                    for procedure in &project.procedures {
                        let branch_in_procedure = procedure.branches.iter().find(|&b| *b == branch.name);
                        if branch_in_procedure.is_none() {
                            continue;
                        }

                        // Kill previous procedure process
                        for unlocked_procedure_thread_connection in &procedure_thread_connections {
                            let procedure_thread_connection = &unlocked_procedure_thread_connection.read().unwrap();
                            if procedure_thread_connection.remote_url == project.url && procedure_thread_connection.branch == branch.name && procedure_thread_connection.procedure_name == procedure.name {
                                info!("Found previous running version. Attempting to send kill message");
                                let sen = &procedure_thread_connection.owner_channel.sender.read().unwrap();
                                sen.send(Command::KillProcedure).expect("Failed to send kill command!");
                                // TODO: Wait for response/timeout
                            }
                        }

                        // Insert new connection
                        procedure_thread_connections.push(Arc::new(RwLock::new(ThreadProcedureConnection::new(project.url.clone(), branch.name.clone(), procedure.name.clone()))));
                        let procedure_connection = procedure_thread_connections.last_mut().unwrap();

                        // Run procedure
                        run_project_procedure(&project, &branch, &procedure, Arc::clone(&procedure_connection)).expect("Procedure failed!");
                    }

                    /*if procedure_immediate_result.is_err() {
                        error!(format!("Error occurred while running procedure: {:?}", procedure_immediate_result));
                    } else {
                        info!("Update most likely succeeded"); // Horribly incorrect
                    }*/
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
