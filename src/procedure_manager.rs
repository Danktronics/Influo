use std::{
    thread,
    process::ExitStatus,
    sync::{Arc, RwLock}
};
use anyhow::Error;
use futures::{select, pin_mut, join, future::FutureExt};
use tokio::{
    process::{Child, ChildStdout, ChildStderr},
    runtime::Builder,
    io::{BufReader, AsyncBufReadExt}
};
use chrono::Utc;

use crate::{
    model::{
        project::{
            Project,
            branch::Branch,
            procedure::{Procedure, AutoRestartPolicy},
        },
        channel::{
            Channel,
            ThreadProcedureConnection,
            message::Command
        }
    },
    system_cmd::{setup_git_repository, run_procedure_command}
};

pub fn run_project_procedure(project: &Project, branch: &Branch, procedure: &Procedure, procedure_thread_connection: Arc<RwLock<ThreadProcedureConnection>>) -> Result<(), Error> {
    let repository_name: String = setup_git_repository(&project.url, procedure.deploy_path.as_ref().unwrap(), &branch.name)?;
    let path = format!("{}/{}/{}", procedure.deploy_path.as_ref().unwrap(), repository_name, branch.name);
    let commands: Vec<String> = procedure.commands.clone();
    let procedure_name = procedure.name.clone();
    let procedure_log = procedure.log.clone();
    let procedure_restart_policy = procedure.auto_restart.clone();

    if !commands.is_empty() {
        thread::spawn(move || {
            let mut success = true;
            let mut current_command_index = 0;
            loop {
                let command = &commands[current_command_index];
    
                info!(format!("[{}] [{}] Running command: {}", procedure_name, path, command));
                let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
                let _guard = runtime.enter();
                let result_child_process = run_procedure_command(&command, &path);
                if result_child_process.is_err() {
                    break;
                }
                let mut child_process: Child = result_child_process.unwrap();
    
                // Print stdout and stderr from child process asynchronously
                if procedure_log.is_some() {
                    let pname: String = procedure_name.clone();
                    let plog: String = procedure_log.clone().unwrap();
                    let p = path.clone();
                    let c = command.clone();
                    let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
                    let stderr = child_process.stderr.take().expect("Child process stderr handle missing");
                    let mut stdout_reader = BufReader::new(stdout);
                    let mut stderr_reader = BufReader::new(stderr);
                    runtime.spawn(async move {
                        join!(read_stdout(&mut stdout_reader, &pname, &p, &c, &plog), read_stderr(&mut stderr_reader, &pname, &p, &c, &plog));
                    });
                }
    
                // Blocks the thread until the child process running the command has exited
                let read_connection = procedure_thread_connection.read().unwrap();
                let child_result = runtime.block_on(manage_child(&mut child_process, &read_connection));
                if !child_result.0 {
                    if let Some(exit_code) = child_result.1 {
                        let should_restart = match &procedure_restart_policy {
                            AutoRestartPolicy::Always => true,
                            AutoRestartPolicy::Never => false,
                            AutoRestartPolicy::ExclusionCodes(excluded_codes) => !excluded_codes.contains(&exit_code),
                            AutoRestartPolicy::InclusionCodes(included_codes) => included_codes.contains(&exit_code)
                        };
                        
                        if !should_restart {
                            match runtime.block_on(child_process.kill()) {
                                Ok(()) => (),
                                Err(_e) => warn!(format!("[{}] Unable to kill child process. It may already be dead.", procedure_name))
                            };
                            info!(format!("[{}] Skipping the remaining commands for project (URL: {}) on branch {} in procedure {}", procedure_name, read_connection.remote_url, read_connection.branch, read_connection.procedure_name));
                            success = false;
                            break;
                        }
                    } else {
                        error!(format!("[{}] Encountered unsuccessful child response with missing exit code", procedure_name));
                        success = false;
                        break;
                    }
                } else {
                    current_command_index += 1;
                }
    
                if commands.len() == current_command_index {
                    break;
                }
            }
            if success {
                info!(format!("[{}] Work completed successfully!", procedure_name));
            } else {
                warn!(format!("[{}] Work did not complete.", procedure_name));
            }
        });
    }

    Ok(())
}

/// Manages a child and returns a future with the result
/// Result.0 is if the command was successful
/// Result.1 is if the command should be rerun
async fn manage_child(child: &mut Child, connection: &ThreadProcedureConnection) -> (bool, Option<i32>) {
    let child_completion_future = complete_child(child).fuse();
    let command_exit = process_commands(&connection.owner_channel).fuse();

    pin_mut!(child_completion_future, command_exit);

    select! {
        (success, exit_code) = child_completion_future => {
            debug!(format!("[{}]: Child exited with code {}", connection.procedure_name, exit_code));
            return (success, Some(exit_code));
        },
        () = command_exit => {
            debug!(format!("[{}]: Terminating due to Command::KillProcedure", connection.procedure_name));
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
async fn process_commands(connection: &Channel<Command>) {
    let rec = &mut connection.receiver.write().unwrap();
    while let Some(msg) = rec.recv().await {
        if std::mem::discriminant(&msg) == std::mem::discriminant(&Command::KillProcedure) {
            break;
        }
    }
}

// STDOUT logging
async fn read_stdout(stdout_buffer: &mut BufReader<ChildStdout>, procedure_name: &str, path: &str, command: &str, log_pattern: &str) {
    let mut stdout_reader = stdout_buffer.lines();
    while let Some(line) = stdout_reader.next_line().await.unwrap() {
        let out: String = log_pattern
            .replace("{name}", procedure_name)
            .replace("{time}", &Utc::now().format("%H:%M:%S").to_string()) // %H:%M:%S can be shortened to %T but that's fine. Additionally, %r will give formatted 12 hour time.
            .replace("{path}", path)
            .replace("{command}", command)
            .replace("{log}", &line);
        info!(out);
    }
}

// STDERR logging
async fn read_stderr(stderr_buffer: &mut BufReader<ChildStderr>, procedure_name: &str, path: &str, command: &str, log_pattern: &str) {
    let mut stderr_reader = stderr_buffer.lines();
    while let Some(line) = stderr_reader.next_line().await.unwrap() {
        let out: String = log_pattern
            .replace("{name}", procedure_name)
            .replace("{time}", &Utc::now().format("%H:%M:%S").to_string()) // same note as stdout
            .replace("{path}", path)
            .replace("{command}", command)
            .replace("{log}", &line);
        error!(out);
    }
}
