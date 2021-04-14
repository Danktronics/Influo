use std::process::ExitStatus;
use std::sync::Arc;
use anyhow::Error;
use futures::future::FutureExt;
use tokio::{
    select, pin, join,
    process::{Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt},
    sync::mpsc::UnboundedReceiver
};
use chrono::Utc;

use crate::{
    constants::DEFAULT_LOG_TEMPLATE,
    model::{
        project::{
            pipeline::Pipeline,
            procedure::{Procedure, AutoRestartPolicy}
        },
        channel::message::Command
    },
    system_cmd::run_procedure_command
};

pub async fn run_procedure(path: Arc<String>, pipeline: Arc<Pipeline>, stage_index: usize, branch_index: usize, procedure: Procedure, mut procedure_receiver: UnboundedReceiver<Command>) -> Result<(), Error> {
    let procedure = Arc::new(procedure);
    
    if !procedure.commands.is_empty() {
        let mut success = true;
        let mut current_command_index = 0;
        loop {
            let command = &procedure.commands[current_command_index];

            info!(format!("[{}] [{}] Running command: {}", procedure.name.as_ref().unwrap(), path, command));
            let result_child_process = run_procedure_command(&command, &path);
            if result_child_process.is_err() {
                break;
            }
            let mut child_process: Child = result_child_process.unwrap();

            // Print stdout and stderr from child process asynchronously
            if pipeline.log.is_some() && pipeline.log.as_ref().unwrap().is_enabled() {
                let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
                let stderr = child_process.stderr.take().expect("Child process stderr handle missing");
                let procedure = Arc::clone(&procedure);
                let path = Arc::clone(&path);
                let pipeline = Arc::clone(&pipeline);
                
                tokio::task::spawn(async move {
                    join!(
                        read_stdout(BufReader::new(stdout), Arc::clone(&procedure), Arc::clone(&path), Arc::clone(&pipeline), stage_index, current_command_index),
                        read_stderr(BufReader::new(stderr), procedure, path, pipeline, stage_index, current_command_index)
                    );
                });
            }

            // Blocks until the child process running the command has exited
            let child_result = manage_child(&mut child_process, procedure.name.as_ref().unwrap(), &mut procedure_receiver).await;
            if !child_result.0 {
                if let Some(exit_code) = child_result.1 {
                    let should_restart = match &procedure.auto_restart {
                        AutoRestartPolicy::Always => true,
                        AutoRestartPolicy::Never => false,
                        AutoRestartPolicy::ExclusionCodes(excluded_codes) => !excluded_codes.contains(&exit_code),
                        AutoRestartPolicy::InclusionCodes(included_codes) => included_codes.contains(&exit_code)
                    };
                    
                    if !should_restart {
                        match child_process.kill().await {
                            Ok(()) => (),
                            Err(_e) => warn!(format!("[{}] Unable to kill child process. It may already be dead.", procedure.name.as_ref().unwrap()))
                        };
                        info!(format!("[{}] Skipping the remaining commands for project on branch {} in procedure {}", procedure.name.as_ref().unwrap(), pipeline.branches[branch_index], procedure.name.as_ref().unwrap()));
                        success = false;
                        break;
                    }
                } else {
                    error!(format!("[{}] Encountered unsuccessful child response with missing exit code", procedure.name.as_ref().unwrap()));
                    success = false;
                    break;
                }
            } else {
                current_command_index += 1;
            }

            if procedure.commands.len() == current_command_index {
                break;
            }
        }
        
        if success {
            info!(format!("[{}] Work completed successfully!", procedure.name.as_ref().unwrap()));
        } else {
            warn!(format!("[{}] Work did not complete.", procedure.name.as_ref().unwrap()));
        }
    }

    Ok(())
}

/// Manages a child and returns a future with the result
/// Result.0 is if the command was successful
/// Result.1 is if the command should be rerun
async fn manage_child(child: &mut Child, procedure_name: &str, procedure_receiver: &mut UnboundedReceiver<Command>) -> (bool, Option<i32>) {
    let child_completion_future = complete_child(child).fuse();
    let command_exit = process_commands(procedure_receiver).fuse();

    pin!(child_completion_future, command_exit);

    select! {
        (success, exit_code) = child_completion_future => {
            debug!(format!("[{}]: Child exited with code {}", procedure_name, exit_code));
            return (success, Some(exit_code));
        },
        () = command_exit => {
            debug!(format!("[{}]: Terminating due to Command::KillProcedure", procedure_name));
            return (false, None);
        },
    }
}

/// Returns a future completed when the child exits
/// Bool indicates whether it exited successfully
/// i32 is status code
async fn complete_child(child: &mut Child) -> (bool, i32) {
    let status_result: Result<ExitStatus, std::io::Error> = child.wait().await;
    if status_result.is_err() {
        return (false, 1);
    }
    let status = status_result.unwrap();
    let success: bool = status.success();
    let raw_code = status.code();
    let exit_code: i32 = if raw_code.is_some() {
        raw_code.unwrap()
    } else {
        1
    };
    return (success, exit_code);
}

/// Processes incoming messages from the updater thread
/// Future will resolve if a KillProcedure is received
async fn process_commands(procedure_receiver: &mut UnboundedReceiver<Command>) {
    while let Some(command) = procedure_receiver.recv().await {
        match command {
            Command::KillProcedure => break,
            _ => ()
        }
    }
}

// TODO: Possibly combine read_stdout and read_stderr for less code duplication
// STDOUT logging
async fn read_stdout(stdout_buffer: BufReader<ChildStdout>, procedure: Arc<Procedure>, path: Arc<String>, pipeline: Arc<Pipeline>, stage_index: usize, command_index: usize) {
    // TODO: Rewrite to be more idiomatic?
    if pipeline.log.is_some() && pipeline.log.as_ref().unwrap().is_enabled() {
        let log_template = if let Some(ref log_template) = procedure.log_template {
            log_template.clone()
        } else if let Some(ref log_template) = pipeline.log.as_ref().unwrap().template {
            log_template.clone()
        } else {
            DEFAULT_LOG_TEMPLATE.to_owned()
        };

        let mut stdout_reader = stdout_buffer.lines();
        while let Some(line) = stdout_reader.next_line().await.unwrap() {
            let out: String = log_template
                .replace("{pipeline_name}", &pipeline.name)
                .replace("{pipeline_stage}", &pipeline.stages[stage_index])
                .replace("{time}", &Utc::now().format("%H:%M:%S").to_string()) // %H:%M:%S can be shortened to %T but that's fine. Additionally, %r will give formatted 12 hour time.
                .replace("{path}", &path)
                .replace("{command}", &procedure.commands[command_index])
                .replace("{message}", &line);
            info!(out);
        }
    } else {
        error!("read_stdout called without log_template");
    }
}

// STDERR logging
async fn read_stderr(stderr_buffer: BufReader<ChildStderr>, procedure: Arc<Procedure>, path: Arc<String>, pipeline: Arc<Pipeline>, stage_index: usize, command_index: usize) {
    if pipeline.log.is_some() && pipeline.log.as_ref().unwrap().is_enabled() {
        let log_template = if let Some(ref log_template) = procedure.log_template {
            log_template.clone()
        } else if let Some(ref log_template) = pipeline.log.as_ref().unwrap().template {
            log_template.clone()
        } else {
            DEFAULT_LOG_TEMPLATE.to_owned()
        };

        let mut stderr_reader = stderr_buffer.lines();
        while let Some(line) = stderr_reader.next_line().await.unwrap() {
            let out: String = log_template
                .replace("{pipeline_name}", &pipeline.name)
                .replace("{pipeline_stage}", &pipeline.stages[stage_index])
                .replace("{time}", &Utc::now().format("%H:%M:%S").to_string()) // %H:%M:%S can be shortened to %T but that's fine. Additionally, %r will give formatted 12 hour time.
                .replace("{path}", &path)
                .replace("{command}", &procedure.commands[command_index])
                .replace("{message}", &line);
            error!(out);
        }
    } else {
        error!("read_stderr called without log_template");
    }
}
