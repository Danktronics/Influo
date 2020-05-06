use std::thread;
use failure::Error;
use futures::{select, pin_mut, future::{Fuse, FusedFuture, FutureExt}};
use futures::executor::block_on;
use tokio::runtime::Builder;
use tokio::process::Child;
use tokio::io::{BufReader, AsyncBufReadExt};
use std::process::ExitStatus;

use crate::model::project::Project;
use crate::model::project::branch::Branch;
use crate::model::channel::{ThreadConnection, ThreadProcedureConnection};
use crate::model::channel::message::{Command, Response};
use crate::system_cmd::{get_remote_git_repository_commits, setup_git_repository, run_procedure_command};

pub fn run_project_procedures(project: &Project, branch: &Branch, procedure_thread_connections: &mut Vec<ThreadProcedureConnection>) -> Result<(), Error> {
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
                let mut runtime = Builder::new().basic_scheduler().enable_all().build().unwrap();
                let result_child_process = runtime.handle().clone().enter(|| run_procedure_command(&command, &path));
                if result_child_process.is_err() {
                    break;
                }
                let mut child_process: Child = result_child_process.unwrap();

                // Print stdout from child process asynchronously
                let p = path.clone();   // thanks Rust
                let c = command.clone();
                let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
                let mut stdout_reader = BufReader::new(stdout).lines();
                runtime.spawn(async move {
                    loop {
                        match stdout_reader.next_line().await {
                            Ok(result) => {
                                println!("uhh");
                                if result.is_some() {
                                    info!(format!("[{}] Command ({}): {}", p, c, result.unwrap()));
                                }
                            },
                            Err(_e) => break,
                        };
                    }
                });

                // Blocks the thread until the child process running the command has exited
                if !runtime.block_on(manage_child(child_process, &procedure_connection)) {
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

/// Manages a child and returns a future with a bool (true if command ran successfully)
async fn manage_child(child: Child, connection: &ThreadProcedureConnection) -> bool {
    let child_completion_future = complete_child(child).fuse();
    let command_exit = process_commands(connection).fuse();

    pin_mut!(child_completion_future, command_exit);

    select! {
        (success, exit_code) = child_completion_future => {
            println!("d");
            let command_log: String = format!("[{}] [{}] {} exited with code {}", connection.remote_url, connection.branch, connection.procedure_name, exit_code);
            if success {
                info!(command_log);
            } else {
                error!(command_log);
            }
            return success;
        },
        () = command_exit => return false,
    }
}

/// Returns a future completed when the child exits
/// Bool indicates whether it exited successfully
/// i32 is status code
async fn complete_child(child: Child) -> (bool, i32) {
    let status_result: Result<ExitStatus, std::io::Error> = child.await; // Blocking
    println!("why");
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

/// 
async fn process_commands(connection: &ThreadProcedureConnection) {
    loop {
        if let Ok(msg) = connection.owner_channel.receiver.try_recv() {
            if std::mem::discriminant(&msg) == std::mem::discriminant(&Command::KillProcedure) {
                info!(format!("[{}] [{}] {}: Terminating due to command", connection.remote_url, connection.branch, connection.procedure_name));
                break;
            }
        }
    }
}
