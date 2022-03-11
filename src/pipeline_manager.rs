use std::sync::Arc;
use std::collections::HashMap;

use futures::future::join_all;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::system_cmd::setup_git_repository;
use crate::procedure_manager::run_procedure;
use crate::model::{
    project::pipeline::{Pipeline, Stage},
    project::procedure::Procedure,
    channel::message::Command
};

pub async fn run_pipeline(
    mut receiver: UnboundedReceiver<Command>,
    project_url: String,
    default_deploy_path: String,
    pipeline: Arc<Pipeline>,
    short_hash: Arc<String>,
    branch_index: usize,
    default_log_path: String
) {
    if let Ok((path, repository_name)) = setup_git_repository(&project_url, pipeline.deploy_path.as_ref().unwrap_or(&default_deploy_path), &pipeline.name, &pipeline.branches[branch_index]).await {
        let path = Arc::new(path);
        let default_log_path = Arc::new(format!("{}/{}", default_log_path, repository_name));

        for (stage_index, stage_name) in pipeline.stages_order.as_ref().unwrap().iter().enumerate() {
            if let Some(stage) = Arc::clone(&pipeline).stages.get(stage_name) {        
                let mut procedures_connection = HashMap::new();
                let mut procedures_handle = Vec::new();

                let mut setup_procedure = |procedure: &Procedure| {
                    let (sender, receiver) = unbounded_channel();
                    let connection_id = match &procedure.name {
                        Some(procedure_name) => procedure_name.clone(),
                        None => pipeline.name.clone()
                    };
    
                    procedures_connection.insert(connection_id, sender);
                    let procedure_future = run_procedure(Arc::clone(&path), Arc::clone(&pipeline), stage_index, branch_index, Arc::clone(&short_hash), procedure.clone(), Arc::clone(&default_log_path), receiver);
                    procedures_handle.push(tokio::task::spawn(procedure_future));
                };

                match stage {
                    Stage::Multiple(procedures) => {
                        for procedure in procedures {
                            setup_procedure(procedure);
                        }
                    },
                    Stage::Single(procedure) => {
                        setup_procedure(procedure);
                    }
                };

                select! {
                    procedure_results = join_all(procedures_handle) => {
                        for result in procedure_results {
                            if result.is_err() || result.unwrap().is_err() {
                                return;
                            }
                        }

                        info!(format!("[{}] [{}] Pipeline finished stage.", pipeline.name, stage_name));
                    },
                    Some(command) = receiver.recv() => {
                        match command {
                            Command::KillProcedure => {
                                debug!(format!("[{}] Pipeline kill command received. Dropping connections and ending task(s).", pipeline.name));
                                for (connection_id, sender) in &procedures_connection {
                                    if sender.send(Command::KillProcedure).is_err() {
                                        error!(format!("[{}] Pipeline failed to kill procedure with ID: {}. Continuing anyway.", pipeline.name, connection_id));
                                    }
                                }
                                break; // TODO: Re-evaluate sending command as dropping has same functionality
                            }
                        }
                    }
                }
            } else {
                error!(format!("[{}] Missing stage configuration! Stage: {}", pipeline.name, stage_name));
            }
        }

        info!(format!("[{}] Pipeline finished.", pipeline.name));
    } else {
        error!(format!("[{}] Failed to setup git repository. Skipping pipeline.", pipeline.name));
    }
}
