use std::thread;
use failure::Error;
use futures::{select, pin_mut, future::{Fuse, FusedFuture, FutureExt}};
use futures::executor::block_on;
use tokio::process::Child;
use tokio::io::{BufReader, AsyncBufReadExt};
use std::process::ExitStatus;

use crate::model::project::Project;
use crate::model::project::branch::Branch;
use crate::model::channel::{ThreadConnection, ThreadProcedureConnection};
use crate::model::channel::message::{Command, Response};
use crate::system_cmd::{get_remote_git_repository_commits, setup_git_repository, run_procedure_command};

pub fn run_project_procedures(project: &Project, branch: &Branch, mut procedure_thread_connections: Vec<ThreadProcedureConnection>) -> Result<(), Error> {
    for procedure in &project.procedures {
        let branch_in_procedure = procedure.branches.iter().find(|&b| *b == branch.name);
        if branch_in_procedure.is_none() {
            continue;
        }
        let repository_name: String = setup_git_repository(&project.url, &procedure.deploy_path, &branch.name)?;
        let path = format!("{}/{}", procedure.deploy_path, repository_name);
        let commands: Vec<String> = procedure.commands.clone();

        let procedure_connection: ThreadProcedureConnection = ThreadProcedureConnection::new(project.url.clone(), branch.name.clone(), procedure.name.clone());
        procedure_thread_connections.push(procedure_connection.clone());

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
                let p = path.clone();   // thanks Rust
                let c = command.clone();
                let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
                let mut stdout_reader = BufReader::new(stdout).lines();
                tokio::spawn(async move {
                    loop {
                        match stdout_reader.next_line().await {
                            Ok(result) => {
                                if result.is_some() {
                                    info!(format!("[{}] Command ({}): {}", p, c, result.unwrap()));
                                }
                            },
                            Err(_e) => break,
                        };
                    }
                });

                // Blocks the thread until the child process running the command has exited
                if !block_on(manage_child(child_process, &procedure_connection)) {
                    info!(format!("Skipping the remaining commands for project (URL: {}) on branch {} in procedure {}", procedure_connection.remote_url, procedure_connection.branch, procedure_connection.procedure_name));
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

async fn manage_child(child: Child, connection: &ThreadProcedureConnection) -> bool {
    let t1 = get_output_on_complete(child).fuse();
    let t2 = terminate_on_command(connection).fuse();

    pin_mut!(t1, t2);

    select! {
        b = t1 => return b,
        b = t2 => return b,
    }
}

async fn get_output_on_complete(child: Child) -> bool {
    let status: ExitStatus = child.await.expect("Oh god what happened"); // blocking
    let out: bool = status.success();
    if !out {
        match status.code() {
            Some(code) => {
                info!(format!("Exited with status code {}", code));
            }
            None => {
                info!("Process terminated by signal");
            }
        };
    }
    return out;
}

async fn terminate_on_command(connection: &ThreadProcedureConnection) -> bool {
    loop {
        if let Ok(msg) = connection.owner_channel.receiver.try_recv() {
            if std::mem::discriminant(&msg) == std::mem::discriminant(&Command::KillProcedure) {
                info!("Terminating command");
                break;
            }
        }
    }
    return false;
}
