// Dependencies
use std::{
    thread,
    time::Duration,
    sync::{Arc, Mutex, RwLock}
};
use anyhow::Error;
use serde_json::Value;

// Project Modules
#[macro_use]
mod logger;
mod model;
mod system_cmd;
mod procedure_manager;
mod filesystem;

#[cfg(feature = "http-api")]
mod api;

use model::{
    Configuration,
    channel::message::Command,
    channel::ThreadProcedureConnection
};
use system_cmd::get_remote_git_repository_commits;
use procedure_manager::run_project_procedure;
use logger::{LOGGER};
use filesystem::read_configuration;

#[cfg(feature = "http-api")]
use api::http::start_http_server;

#[tokio::main]
async fn main() -> Result<(), Error> {
    info!("Influo is running!");

    // Load Configuration
    let raw_config: Result<Value, Error> = read_configuration();
    if let Err(error) = raw_config {
        error!("Configuration not found");
        return Err(error);
    }

    let mut configuration: Configuration = serde_json::from_value(raw_config.unwrap())?;
    for project in &mut configuration.projects {
        project.persistent = true;
        
        for mut procedure in &mut project.procedures {
            if procedure.deploy_path.is_none() {
                procedure.deploy_path = Some(configuration.default_deploy_path.clone());
                procedure.persistent = true;
            }
        }
    }

    LOGGER.lock().unwrap().set_log_level(configuration.log_level);

    let protected_configuration = Arc::new(Mutex::new(configuration));

    #[cfg(feature = "http-api")]
    start_http_server(Arc::clone(&protected_configuration))?;

    // Start the updater thread
    let thread_join_handle: thread::JoinHandle<()> = setup_updater_thread(protected_configuration);
    thread_join_handle.join().unwrap();

    Ok(())
}

/// Spawns the updater thread for checking updates and controlling procedures
/// Interval should be in milliseconds
fn setup_updater_thread(configuration: Arc<Mutex<Configuration>>) -> thread::JoinHandle<()> {
    info!("Spawning updater thread");

    let mut procedure_thread_connections: Vec<Arc<RwLock<ThreadProcedureConnection>>> = Vec::new();

    // let updater_projects_ref = Arc::clone(&projects);
    thread::spawn(move || {
        loop {
            let interval;
            {
                let mut configuration = configuration.lock().unwrap();
                interval = configuration.update_interval;
                debug!("Checking project repositories for updates");
                for project in &mut *configuration.projects {
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
                                    info!(format!("[{}] Found previous running version. Attempting to send kill message", procedure.name));
                                    let sen = &procedure_thread_connection.owner_channel.sender.read().unwrap();
                                    sen.send(Command::KillProcedure).expect("Failed to send kill command!");
                                    // TODO: Wait for response/timeout
                                }
                            }
    
                            // Insert new connection
                            procedure_thread_connections.push(Arc::new(RwLock::new(ThreadProcedureConnection::new(project.url.clone(), branch.name.clone(), procedure.name.clone()))));
                            let procedure_connection = procedure_thread_connections.last_mut().unwrap();
    
                            // Run procedure
                            run_project_procedure(&project, &branch, &procedure, Arc::clone(&procedure_connection)).expect("Procedure failed due to a git error!");
                        }
                    }
                    project.update_branches(branches);
                }
            }

            debug!(format!("Updater thread sleeping for {} seconds", interval));
            thread::sleep(Duration::from_secs(interval as u64));
        }
    })
}
