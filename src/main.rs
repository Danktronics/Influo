// Dependencies
use std::{
    time::Duration,
    sync::{Arc, Mutex}
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
    channel::ProcedureConnection
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
    let procedure_thread_connections: Arc<Mutex<Vec<ProcedureConnection>>> = Arc::new(Mutex::new(Vec::new()));

    #[cfg(feature = "http-api")]
    start_http_server(Arc::clone(&protected_configuration), Arc::clone(&procedure_thread_connections))?;

    // Start the updater
    setup_updater(protected_configuration, procedure_thread_connections).await;

    Ok(())
}

/// Setups the updater for checking updates and controlling procedures
async fn setup_updater(configuration: Arc<Mutex<Configuration>>, procedure_thread_connections: Arc<Mutex<Vec<ProcedureConnection>>>) {
    info!("Spawning updater thread");

    loop {
        let interval;
        {
            let mut configuration = configuration.lock().unwrap();
            let mut procedure_connections = procedure_thread_connections.lock().unwrap();
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
                        for unlocked_procedure_thread_connection in &*procedure_connections {
                            let procedure_thread_connection = &unlocked_procedure_thread_connection;
                            if procedure_thread_connection.remote_url == project.url && procedure_thread_connection.branch == branch.name && procedure_thread_connection.procedure_name == procedure.name {
                                info!(format!("[{}] Found previous running version. Attempting to send kill message", procedure.name));
                                procedure_thread_connection.sender.send(Command::KillProcedure).expect("Failed to send kill command!");
                                // TODO: Wait for response/timeout
                            }
                        }

                        // Insert new connection
                        let (procedure_connection, receiver) = ProcedureConnection::new(project.url.clone(), branch.name.clone(), procedure.name.clone());
                        procedure_connections.push(procedure_connection);

                        // Run procedure
                        run_project_procedure(project.url.clone(), branch.name.clone(), procedure.clone(), receiver).expect("Procedure failed due to a git error!");
                    }
                }
                project.update_branches(branches);
            }
        }

        debug!(format!("Updater thread sleeping for {} seconds", interval));
        tokio::time::sleep(Duration::from_secs(interval as u64)).await;
    }
}
