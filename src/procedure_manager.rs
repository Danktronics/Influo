use std::{
    thread,
    process::ExitStatus,
    sync::{Arc, RwLock}
};
use failure::Error;
use futures::{select, pin_mut, future::FutureExt};
use tokio::{
    process::Child,
    runtime::Builder,
    io::{BufReader, AsyncBufReadExt}
};
use crate::{
    model::{
        project::{
            Project,
            branch::Branch,
            procedure::Procedure,
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
    let repository_name: String = setup_git_repository(&project.url, &procedure.deploy_path, &branch.name)?;
    let path = format!("{}/{}/{}", procedure.deploy_path, repository_name, branch.name);
    let commands: Vec<String> = procedure.commands.clone();
    let procedure_name = procedure.name.clone();

    thread::spawn(move || {
        let mut success = true;
        for command in commands {
            info!(format!("[{}] [{}] Running command: {}", procedure_name, path, command));
            let mut runtime = Builder::new().threaded_scheduler().enable_all().build().unwrap();
            let result_child_process = runtime.handle().enter(|| run_procedure_command(&command, &path));
            if result_child_process.is_err() {
                break;
            }
            let mut child_process: Child = result_child_process.unwrap();

            // Print stdout from child process asynchronously
            let pname: String = procedure_name.clone();
            let p = path.clone();   // thanks Rust
            let c = command.clone();
            let stdout = child_process.stdout.take().expect("Child process stdout handle missing");
            let mut stdout_reader = BufReader::new(stdout).lines();
            runtime.spawn(async move {
                while let Some(line) = stdout_reader.next_line().await.unwrap() {
                    info!(format!("[{}] [{}] Command ({}): {}", pname, p, c, line));
                }
            });

            // Blocks the thread until the child process running the command has exited
            let read_connection = procedure_thread_connection.read().unwrap();
            if !runtime.block_on(manage_child(&mut child_process, &read_connection)) {
                child_process.kill().expect("Failed to kill child process!");
                info!(format!("[{}] Skipping the remaining commands for project (URL: {}) on branch {} in procedure {}", procedure_name, read_connection.remote_url, read_connection.branch, read_connection.procedure_name));
                success = false;
                break;
            }
        }
        if success {
            info!(format!("[{}] Work completed successfully!", procedure_name));
        } else {
            error!(format!("[{}] Work did not complete.", procedure_name));
        }
    });

    Ok(())
}

/// Manages a child and returns a future with a bool (true if command ran successfully)
async fn manage_child(child: &mut Child, connection: &ThreadProcedureConnection) -> bool {
    let child_completion_future = complete_child(child).fuse();
    let command_exit = process_commands(&connection.owner_channel).fuse();

    pin_mut!(child_completion_future, command_exit);

    select! {
        (success, exit_code) = child_completion_future => {
            /*let command_log: String = format!("[{}] [{}] {} exited with code {}", connection.remote_url, connection.branch, connection.procedure_name, exit_code);
            if success {
                info!(command_log);
            } else {
                error!(command_log);
            }*/
            return success;
        },
        () = command_exit => return false,
    }
}

/// Returns a future completed when the child exits
/// Bool indicates whether it exited successfully
/// i32 is status code
async fn complete_child(child: &mut Child) -> (bool, i32) {
    let status_result: Result<ExitStatus, std::io::Error> = child.await; // Blocking
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

/// yes
async fn process_commands(connection: &Channel<Command>) {
    let rec = &mut connection.receiver.write().unwrap();
    while let Some(msg) = rec.recv().await {
        if std::mem::discriminant(&msg) == std::mem::discriminant(&Command::KillProcedure) {
            //info!(format!("[{}] [{}] {}: Terminating due to command", connection.remote_url, connection.branch, connection.procedure_name));
            break;
        }
    }
}
