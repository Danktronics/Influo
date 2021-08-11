// Dependencies
use std::{
    time::Duration,
    sync::{Arc, Mutex}
};
use anyhow::Error;
use serde_json::Value;

// Project Modules
mod constants;
mod error;
#[macro_use]
mod logger;
mod model;
mod system_cmd;
mod pipeline_manager;
mod procedure_manager;
mod util;

#[cfg(feature = "http-api")]
mod api;

use model::{
    Configuration,
    channel::message::Command,
    channel::PipelineConnection,
    project::pipeline::Condition
};
use system_cmd::get_remote_git_repository_commits;
use logger::LOGGER;
use pipeline_manager::run_pipeline;
use util::filesystem::read_configuration;

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
    // Initially set all projects to be persistent regardless of user settings as the configuration is from the disk
    for project in &mut configuration.projects {
        project.persistent = true;
        
        for pipeline in &mut project.pipelines {
            pipeline.persistent = true;
        }
    }

    LOGGER.lock().unwrap().set_log_level(configuration.log_level);

    let protected_configuration = Arc::new(Mutex::new(configuration));
    let procedure_thread_connections: Arc<Mutex<Vec<PipelineConnection>>> = Arc::new(Mutex::new(Vec::new()));

    #[cfg(feature = "http-api")]
    start_http_server(Arc::clone(&protected_configuration), Arc::clone(&procedure_thread_connections))?;

    // Start the updater
    setup_updater(protected_configuration, procedure_thread_connections).await;

    Ok(())
}

/// Setups the updater for checking updates and controlling procedures
async fn setup_updater(configuration: Arc<Mutex<Configuration>>, procedure_thread_connections: Arc<Mutex<Vec<PipelineConnection>>>) {
    info!("Starting updater");

    loop {
        let interval;
        {
            let mut configuration = configuration.lock().unwrap();
            let mut pipeline_connections = procedure_thread_connections.lock().unwrap();
            interval = configuration.update_interval;
            debug!("Checking project repositories for updates");
            for project_index in 0..configuration.projects.len() {
                let possible_new_branches;
                {
                    let project = &configuration.projects[project_index];
                    
                    match get_remote_git_repository_commits(&project.url).await {
                        Ok(branches) => {
                            for branch in &branches {
                                let short_hash: String = branch.latest_commit_hash.chars().take(5).collect();
                                debug!(format!("Current branch is {}. Current short commit hash is {}", branch.name, short_hash));
                                let branch_search = project.branches.iter().find(|&b| b.name == branch.name);
                                if branch_search.is_some() && branch_search.unwrap().latest_commit_hash == branch.latest_commit_hash {
                                    continue;
                                }
            
                                info!(format!("Updating to commit {} in the {} branch...", short_hash, branch.name));
                                let short_hash = Arc::new(short_hash);
                                for pipeline in &project.pipelines {
                                    if let Some(branch_index) = pipeline.branches.iter().position(|b| *b == branch.name) {
                                        if pipeline.condition == Condition::Automatic {
                                            // Kill previous procedure process
                                            for pipeline_connection in &*pipeline_connections {
                                                if pipeline_connection.remote_url == project.url && pipeline_connection.branch_name == branch.name && pipeline_connection.pipeline_name == pipeline.name {
                                                    info!(format!("[{}] Found previous running version. Attempting to send kill message", pipeline.name));
                                                    if pipeline_connection.send(Command::KillProcedure).is_err() {
                                                        error!(format!("[{}] Attempted to kill previous pipeline task, but failed. Continuing anyway.", pipeline.name));
                                                    }
                                                    // TODO: Wait for response/timeout
                                                }
                                            }
                
                                            if pipeline.stages.is_none() || !pipeline.stages.unwrap().is_empty() {
                                                let (pipeline_connection, receiver) = PipelineConnection::new(project.url.clone(), branch.name.clone(), pipeline.name.clone());
                                                pipeline_connections.push(pipeline_connection);
                                                let default_deploy_path = configuration.default_deploy_path.clone();
                                                let project_url = project.url.clone();
                                                let pipeline = Arc::new(pipeline.clone());
                                                let short_hash = Arc::clone(&short_hash);
                                                let default_log_path = configuration.default_log_path.clone(); // TODO: Revisit possible unnecessary clone (along with all its uses)
                
                                                tokio::task::spawn(run_pipeline(receiver, project_url, default_deploy_path, pipeline, short_hash, branch_index, default_log_path));
                                            }
                                        }
                                    } else {
                                        continue;
                                    }
                                }
                            }
        
                            possible_new_branches = Some(branches);
                        },
                        Err(error) => {
                            error!(format!("Failed to query commits for project with url {} and error:\n{}", project.url, error));
                            continue;
                        }
                    }
                }

                if let Some(new_branches) = possible_new_branches {
                    configuration.projects[project_index].update_branches(new_branches);
                }
            }
        }

        debug!(format!("Updater thread sleeping for {} seconds", interval));
        tokio::time::sleep(Duration::from_secs(interval as u64)).await;
    }
}
