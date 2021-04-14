// Dependencies
use std::{
    time::Duration,
    sync::{Arc, Mutex},
    collections::HashMap
};
use anyhow::Error;
use serde_json::Value;
use futures::future::join_all;
use tokio::select;
use tokio::sync::mpsc::unbounded_channel;

// Project Modules
mod constants;
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
    channel::PipelineConnection,
    project::pipeline::Condition
};
use system_cmd::{get_remote_git_repository_commits, setup_git_repository};
use procedure_manager::run_procedure;
use logger::LOGGER;
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
                let mut possible_new_branches = None;
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
                
                                            if !pipeline.stages.is_empty() {
                                                let (pipeline_connection, mut receiver) = PipelineConnection::new(project.url.clone(), branch.name.clone(), pipeline.name.clone());
                                                pipeline_connections.push(pipeline_connection);
                                                let default_deploy_path = configuration.default_deploy_path.clone();
                                                let project_url = project.url.clone();
                                                let pipeline = Arc::new(pipeline.clone());
                
                                                tokio::task::spawn(async move {
                                                    if let Ok(path) = setup_git_repository(&project_url, pipeline.deploy_path.as_ref().unwrap_or(&default_deploy_path), &pipeline.name, &pipeline.branches[branch_index]).await {
                                                        let path = Arc::new(path);
                                                        for (stage_index, stage) in pipeline.stages.iter().enumerate() {
                                                            if let Some(procedures) = Arc::clone(&pipeline).procedures.get(stage) {        
                                                                let mut procedures_connection = HashMap::new();
                                                                let mut procedures_future = Vec::new();
                                                                for procedure in procedures {
                                                                    let (sender, receiver) = unbounded_channel();
                                                                    procedures_connection.insert(procedure.name.clone().unwrap(), sender);
                                                                    let procedure_future = run_procedure(Arc::clone(&path), Arc::clone(&pipeline), branch_index, stage_index, procedure.clone(), receiver);
                                                                    procedures_future.push(tokio::task::spawn(async move {
                                                                        procedure_future.await; // TODO: Possibly handle Result (which is useless for now)
                                                                    }));
                                                                }
                    
                                                                select! {
                                                                    _ = join_all(procedures_future) => {
                                                                        info!(format!("[{}] Pipeline finished all stages.", pipeline.name));
                                                                    },
                                                                    Some(command) = receiver.recv() => {
                                                                        match command {
                                                                            Command::KillProcedure => {
                                                                                debug!(format!("[{}] Pipeline kill command received. Dropping connections and ending task.", pipeline.name));
                                                                                break; // TODO: Re-evaluate sending command as dropping has same functionality
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                error!(format!("[{}] Missing stage configuration! Stage: {}", pipeline.name, stage));
                                                            }
                                                        }
                                                    } else {
                                                        error!(format!("[{}] Failed to setup git repository. Skipping pipeline.", pipeline.name));
                                                    }
                                                });
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
