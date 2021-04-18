use std::process::ExitStatus;
use std::sync::Arc;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use futures::future::FutureExt;
use tokio::{
    select, pin,
    process::{Child, ChildStdout, ChildStderr},
    io::{BufReader, AsyncBufReadExt},
    sync::mpsc::UnboundedReceiver
};
use chrono::{Utc, DateTime};

use crate::{
    constants::DEFAULT_LOG_TEMPLATE,
    error::ProcedureError,
    model::{
        project::{
            pipeline::Pipeline,
            procedure::{Procedure, AutoRestartPolicy}
        },
        channel::message::Command
    },
    system_cmd::run_procedure_command
};

// TODO: Update Influo logging (currently panics if no procedure name)
pub async fn run_procedure(
    path: Arc<String>,
    pipeline: Arc<Pipeline>,
    stage_index: usize,
    branch_index: usize,
    commit_hash: Arc<String>,
    procedure: Procedure,
    default_log_path: Arc<String>,
    mut procedure_receiver: UnboundedReceiver<Command>
) -> Result<(), ProcedureError> {
    let procedure = Arc::new(procedure);
    let log_identifier = format!("[{}] [{}] [{}]", pipeline.name, pipeline.stages[stage_index], procedure.name.as_ref().unwrap_or(&pipeline.name));
    let start_time = Arc::new(Utc::now());
    
    if !procedure.commands.is_empty() {
        let mut current_command_index = 0;
        loop {
            let command = &procedure.commands[current_command_index];

            info!(format!("{} Running command: {}", log_identifier, command));
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
                let commit_hash = Arc::clone(&commit_hash);
                let default_log_path = Arc::clone(&default_log_path);
                let start_time = Arc::clone(&start_time);
                
                // TODO: Revisit re-initializing log read every command
                tokio::task::spawn(read_standard_streams(BufReader::new(stdout), BufReader::new(stderr), procedure, path, pipeline, stage_index, current_command_index, branch_index, commit_hash, default_log_path, start_time));
            }

            // Blocks until the child process running the command has exited
            let child_result = manage_child(&mut child_process, procedure.name.as_ref().unwrap(), &mut procedure_receiver).await;
            if !child_result.0 {
                if let Some(exit_code) = child_result.1 {
                    let should_restart = match &procedure.auto_restart {
                        Some(auto_restart) => match auto_restart {
                            AutoRestartPolicy::Always => true,
                            AutoRestartPolicy::Never => false,
                            AutoRestartPolicy::ExclusionCodes(excluded_codes) => !excluded_codes.contains(&exit_code),
                            AutoRestartPolicy::InclusionCodes(included_codes) => included_codes.contains(&exit_code)
                        },
                        None => false
                    };
                    
                    if !should_restart {
                        match child_process.kill().await {
                            Ok(()) => (),
                            Err(_e) => warn!(format!("{} [{}] Unable to kill child process. It may already be dead.", log_identifier, command))
                        };
                        info!(format!("{} Skipping the remaining commands", log_identifier));
                        return Err(ProcedureError::ChildKillFail);
                    }
                } else {
                    error!(format!("{} [{}] Encountered unsuccessful child response with missing exit code", log_identifier, command));
                    return Err(ProcedureError::ChildEndMissingCloseCode);
                }
            } else {
                current_command_index += 1;
            }

            if procedure.commands.len() == current_command_index {
                break;
            }
        }
        
        info!(format!("{} Work completed successfully!", log_identifier));
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

async fn read_standard_streams(
    stdout_buffer: BufReader<ChildStdout>,
    stderr_buffer: BufReader<ChildStderr>,
    procedure: Arc<Procedure>,
    path: Arc<String>,
    pipeline: Arc<Pipeline>,
    stage_index: usize,
    command_index: usize,
    branch_index: usize,
    commit_hash: Arc<String>,
    default_log_path: Arc<String>,
    start_time: Arc<DateTime<Utc>> // TODO: Fix unnecessary reference
) {
    if pipeline.log.is_some() && pipeline.log.as_ref().unwrap().is_enabled() {
        let log_template = if let Some(ref log_template) = procedure.log_template {
            log_template.clone()
        } else if let Some(ref log_template) = pipeline.log.as_ref().unwrap().template {
            log_template.clone()
        } else {
            DEFAULT_LOG_TEMPLATE.to_owned()
        };

        let mut stdout_reader = stdout_buffer.lines();
        let mut stderr_reader = stderr_buffer.lines();

        enum StreamType {
            Stdout,
            Stderr
        }
        
        let log_identifier = format!("[{}] [{}] [{}] [{}]", pipeline.name, pipeline.stages[stage_index], procedure.name.as_ref().unwrap_or(&pipeline.name), procedure.commands[command_index]);

        let mut file_log_stream = None;
        if let Some(ref log) = pipeline.log {
            if log.save_to_file.unwrap_or(false) {
                let formatted_date = Utc::now().format("%Y%m%d").to_string();
                let path = if let Some(ref path) = log.file_path {
                    format!("{}/{}", path, &pipeline.branches[branch_index])
                } else {
                    format!("{}/{}/{}", default_log_path, &pipeline.name, &pipeline.branches[branch_index])
                };

                match fs::create_dir_all(&path) {
                    Ok(()) => {
                        match OpenOptions::new().append(true).create(true).open(format!("{}/{}_{}_{}.log", path, commit_hash, procedure.name.as_ref().unwrap_or(&pipeline.name), formatted_date)) { // TODO: Possibly incrementing counter in file name
                            Ok(file) => file_log_stream = Some(file),
                            Err(error) => error!(format!("{} Failed to create log file. Error: {}", log_identifier, error))
                        }
                    },
                    Err(error) => error!(format!("{} Failed to setup log folder structure. Error: {}", log_identifier, error))
                }
            }
        }

        loop {
            let stream_type;
            let stream_data;

            select! {
                Ok(Some(line)) = stdout_reader.next_line() => {
                    stream_type = StreamType::Stdout;
                    stream_data = line;
                },
                Ok(Some(line)) = stderr_reader.next_line() => {
                    stream_type = StreamType::Stderr;
                    stream_data = line;
                },
                else => break
            }

            let duration = Utc::now() - *start_time;

            let log = log_template
                .replace("{pipeline_name}", &pipeline.name)
                .replace("{pipeline_stage}", &pipeline.stages[stage_index])
                .replace("{time}", &format!("{:02}:{:02}:{:02}", duration.num_hours(), duration.num_minutes(), duration.num_seconds()))
                .replace("{path}", &path)
                .replace("{command}", &procedure.commands[command_index])
                .replace("{message}", &stream_data);

            if pipeline.log.as_ref().unwrap().console.unwrap_or(false) {
                match stream_type {
                    StreamType::Stdout => info!(log),
                    StreamType::Stderr => error!(log)
                }
            }

            if let Some(ref mut file_stream) = file_log_stream {
                if let Err(error) = writeln!(file_stream, "{}", log) {
                    error!(format!("{} Failed to save log to file. Closing stream and aborting file logging. Error: {}", log_identifier, error));
                    file_log_stream = None;
                }
            }
        }
    } else {
        error!("read_standard_streams called without log_template");
    }
}
