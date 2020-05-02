// external+builtin dependencies
use std::fs;
use std::io::{Read, BufReader, BufRead};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::process::{Child, ExitStatus};
use failure::{Error, err_msg};
use serde_json::Value;
use crossbeam_channel::{unbounded, Sender, Receiver};

// project dependencies
mod model;
mod system_cmd;

use model::project::Project;
// use model::project::messages::Messages; // maybe future
use model::project::branch::Branch;
use system_cmd::{get_remote_git_repository_commits, setup_git_repository, run_procedure_command};

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;

    let raw_projects: &Value = &config["projects"];
    if !raw_projects.is_array() {
        return Err(err_msg("Projects is invalid"));
    }
    let raw_projects_array: &Vec<Value> = raw_projects.as_array().unwrap();
    let projects: Arc<Mutex<Vec<Project>>> = Arc::new(Mutex::new(Vec::new()));
    for raw_project in raw_projects_array {
        let mut temp_projects = projects.lock().unwrap();
        temp_projects.push(Project::new(&raw_project["url"], &raw_project["procedures"], &config["default_deploy_path"])?);
    }

    let update_interval: &Value = &config["update_interval"];
    if update_interval.is_null() || !update_interval.is_number() {
        setup_updater_thread(30, projects);
    } else {
        let interval: Option<u64> = update_interval.as_u64();
        if interval.is_none() || interval.unwrap() > u32::MAX as u64 {
            panic!("The integer provided exceeded the u32 max");
        }
        let join_handle: thread::JoinHandle<()> = setup_updater_thread(interval.unwrap() as u32 * 1000, projects);
        join_handle.join().unwrap();
    }

    Ok(())
}

/// Interval should be in milliseconds
fn setup_updater_thread(interval: u32, projects: Arc<Mutex<Vec<Project>>>) -> thread::JoinHandle<()> {
    println!("Spawning updater thread");
    let updater_projects_ref = Arc::clone(&projects);
    thread::spawn(move || {
        let mut temp_projects = updater_projects_ref.lock().unwrap();
        let (s, r) = unbounded();
        loop {
            // println!("Checking project repositories for updates"); // debug
            for project in &mut *temp_projects { // Uhhh
                let query_result = get_remote_git_repository_commits(&project.url);
                if query_result.is_err() {
                    println!("Failed to query commits for project with url {} and error:\n{}", project.url, query_result.err().unwrap());
                    continue;
                }

                let branches = query_result.unwrap();
                for branch in &branches {
                    let mut short_hash = branch.latest_commit_hash.clone();
                    short_hash.truncate(7);
                    // println!("Current branch is {}. Current short commit hash is {hash}.", branch.name, hash = short_hash); // debug
                    let cached_branch = project.branches.iter().find(|&b| b.name == branch.name);
                    if cached_branch.is_some() && cached_branch.unwrap().latest_commit_hash == branch.latest_commit_hash {
                        continue;
                    }
                    // else
                    println!("Updating to commit {hash} in \"{branch}\" branch...", hash = short_hash, branch = branch.name);
                    s.send(Messages::Terminate);
                    assert_eq!(r.recv(), Messages::Terminated);
                    let procedure_immediate_result = run_project_procedures(&project, &branch, s, r);
                    if procedure_immediate_result.is_err() {
                        println!("Error occurred while running procedure: {:?}", procedure_immediate_result);
                    } else {
                        println!("Update succeeded.")
                    }
                }

                project.update_branches(branches);
            }
            thread::sleep(Duration::from_millis(interval as u64));
        }
    })
}

fn run_project_procedures(project: &Project, branch: &Branch, s: crossbeam_channel::Sender, r: crossbeam_channel::Receiver) -> Result<(), Error> {
    for procedure in &project.procedures {
        let branch_in_procedure = procedure.branches.iter().find(|&b| *b == branch.name);
        if branch_in_procedure.is_none() {
            continue;
        }

        let repository_name: String = setup_git_repository(&project.url, &procedure.deploy_path)?;
        let path = format!("{}/{}", procedure.deploy_path, repository_name);
        let commands: Vec<String> = procedure.commands.clone();

        if r.recv() == Messages::Test {
            // child_process.kill().except("Command was not running.");
            println!("Received test message")
        }

        thread::spawn(move || {
            for command in &commands {
                println!("[{}] Running command: {}", path, command);
                let result_child_process = run_procedure_command(command, &path);
                if result_child_process.is_err() {
                    break;
                }

                // Print std from child process
                let mut child_process: Child = result_child_process.unwrap();
                log_child_output(&mut child_process, &path, &command);
            }
        });
    }

    Ok(())
}

fn log_child_output(child_process: &mut Child, path: &str, command: &str) {
    let stdout = child_process.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for line in stdout_lines {
        println!("[{}] Command ({}): {}", path, command, line.unwrap());
    }
    
    /*println!("{:?}", child_process.stdout);
    match child_process.stdout {
        Some(ref mut out) => {
            let mut output_string = String::new();
            match out.read_to_string(&mut output_string) {
                Ok(_) => {
                    println!("{}", output_string);
                    match child_process.try_wait() {
                        Ok(Some(status)) => println!("[{}] Command ({}) exited with {}", path, command, status),
                        Ok(None) => log_child_output(child_process, path, command),
                        Err(e) => {
                            println!("Child wait error (very bad) error: {}", e);
                            return;
                        },
                    }
                },
                Err(_) => return,
            };
        }
        None => {
            println!("{:?}", child_process.try_wait());
            match child_process.try_wait() {
                Ok(Some(status)) => println!("[{}] Command ({}) exited with {}", path, command, status),
                Ok(None) => log_child_output(child_process, path, command),
                Err(e) => {
                    println!("Child wait error (very bad) error: {}", e);
                    return;
                },
            }
        },
    }*/
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}

enum Messages {
    Test, // dev
    Terminate,
    Terminated,
}
