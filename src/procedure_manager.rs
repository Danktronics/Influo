use std::thread;
use failure::Error;
use tokio::process::Child;
use tokio::io::{BufReader, AsyncBufReadExt};

use crate::model::project::Project;
use crate::model::project::branch::Branch;
use crate::model::channel::{ThreadConnection, ThreadProcedureConnection};
use crate::model::channel::message::{Command, Response};
use crate::system_cmd::{get_remote_git_repository_commits, setup_git_repository, run_procedure_command};

pub fn run_project_procedures(project: &Project, branch: &Branch, procedure_thread_connections: Vec<ThreadProcedureConnection>) -> Result<(), Error> {
    for procedure in &project.procedures {
        let branch_in_procedure = procedure.branches.iter().find(|&b| *b == branch.name);
        if branch_in_procedure.is_none() {
            continue;
        }
        let repository_name: String = setup_git_repository(&project.url, &procedure.deploy_path, &branch.name)?;
        let path = format!("{}/{}", procedure.deploy_path, repository_name);
        let commands: Vec<String> = procedure.commands.clone();

        let procedure_connection: ThreadProcedureConnection = ThreadProcedureConnection::new(project.url, branch.name, procedure.name);
        procedure_thread_connections.push(procedure_connection);

        thread::spawn(move || {
            let mut success = true;
            for command in commands {
                info!(format!("[{}] Running command: {}", path, command));
                let result_child_process = run_procedure_command(&command, &path);
                if result_child_process.is_err() {
                    break;
                }
                let mut child_process: Child = result_child_process.unwrap();

                // Print stdout from child process asynchronously
                tokio::spawn(async {
                    let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
                    let mut stdout_reader = BufReader::new(stdout).lines();
                    loop {
                        let result = stdout_reader.next_line().await;
                        if (result.is_err()) {
                            break;
                        }
                        if result.unwrap().is_some() {
                            info!(format!("[{}] Command ({}): {}", path, command, result.unwrap().unwrap()));
                        }
                    }
                });

                // Blocks the thread until the child process running the command has exited
                if !manage_child(&mut child_process, &procedure_connection) {
                    info!(format!("Skipping the remaining commands for project (URL: {}) on branch {} in procedure {}", project.url, branch.name, procedure.name));
                    success = false;
                    break;
                }
            }
            if success {
                info!("Work completed successfully!");
            }
        });
        break;
    }

    Ok(())
}

fn manage_child(child: &mut Child, connection: &ThreadProcedureConnection) -> bool {
    loop {
        let possible_status = child.try_wait().unwrap();
        if !possible_status.is_none() {
            let status = possible_status.unwrap();
            if status.success() {
                return true;
            }
            match status.code() {
                Some(code) => {
                    info!(format!("Exited with status code {}", code));
                    return false;
                }
                None => {
                    info!("Process terminated by signal");
                    return false;
                }
            };
        }
        if let Ok(msg) = connection.child_channel.receiver.try_recv() {
            if msg == Command::KillProcedure {
                info!("Terminating command");
                child.kill().expect("Command was not running");
                return false;
            }
        }
    }
}