// external+builtin dependencies
use std::fs;
use std::io::{BufReader, BufRead};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::process::Child;
use failure::{Error, err_msg};
use serde_json::Value;
use crossbeam_channel::{unbounded, Receiver};

// project dependencies
mod model;
mod system_cmd;
#[macro_use]
mod logger;

use model::project::Project;
// use model::project::messages::Messages; // maybe future
use model::project::branch::Branch;
use system_cmd::{get_remote_git_repository_commits, setup_git_repository, run_procedure_command};
use logger::{LOGGER, Logger};

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;
    if config["log_level"].is_string() {
        LOGGER.lock().unwrap().set_log_level(Logger::string_to_log_level(&config["log_level"].as_str().unwrap()));
    }

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
    info!("Spawning updater thread");
    let updater_projects_ref = Arc::clone(&projects);
    let (s, r) = unbounded();
    let mut send_term: bool = false;
    thread::spawn(move || {
        let mut temp_projects = updater_projects_ref.lock().unwrap();
        loop {
            thread::sleep(Duration::from_millis(interval as u64));
            info!("Checking project repositories for updates");
            // println!("Checking project repositories for updates"); // debug
            for project in &mut *temp_projects { // Uhhh
                let query_result = get_remote_git_repository_commits(&project.url);
                if query_result.is_err() {
                    error!(format!("Failed to query commits for project with url {} and error:\n{}", project.url, query_result.err().unwrap()));
                    continue;
                }

                let branches = query_result.unwrap();
                for branch in &branches {
                    let mut short_hash = branch.latest_commit_hash.clone();
                    short_hash.truncate(7);
                    info!(format!("Current branch is {}. Current short commit hash is {hash}.", branch.name, hash = short_hash));
                    let cached_branch = project.branches.iter().find(|&b| b.name == branch.name);
                    if cached_branch.is_some() && cached_branch.unwrap().latest_commit_hash == branch.latest_commit_hash {
                        continue;
                    }
                    // else

                    info!(format!("Updating to commit {hash} in \"{branch}\" branch...", hash = short_hash, branch = branch.name));
                    if send_term {
                        s.send(Messages::Terminate).expect("Unable to send terminate signal!");
                    } else {
                        send_term = true;
                    }
                    let procedure_immediate_result = run_project_procedures(&project, &branch, r.clone());

                    if procedure_immediate_result.is_err() {
                        error!(format!("Error occurred while running procedure: {:?}", procedure_immediate_result));
                    } else {
                        info!("Update succeeded.")
                    }
                }

                project.update_branches(branches);
            }
            thread::sleep(Duration::from_millis(interval as u64));
        }
    })
}

fn run_project_procedures(project: &Project, branch: &Branch, r1: Receiver<Messages>) -> Result<(), Error> {
    for procedure in &project.procedures {
        let branch_in_procedure = procedure.branches.iter().find(|&b| *b == branch.name);
        if branch_in_procedure.is_none() {
            continue;
        }
        let repository_name: String = setup_git_repository(&project.url, &procedure.deploy_path)?;
        let path = format!("{}/{}", procedure.deploy_path, repository_name);
        let commands: Vec<String> = procedure.commands.clone();
        let r = r1.clone();

        thread::spawn(move || {
            for command in &commands {
                info!(format!("[{}] Running command: {}", path, command));
                let result_child_process = run_procedure_command(command, &path);
                if result_child_process.is_err() {
                    break;
                }

                // Print std from child process
                let mut child_process: Child = result_child_process.unwrap();
                if !log_child_output(&mut child_process, &path, &command, &r){
                    println!("Skipping the remaining commands.");
                    break;
                }
            }
            println!("Work appeared to complete successfully!")
        });
    }

    Ok(())
}

fn log_child_output(child_process: &mut Child, path: &str, command: &str, r: &Receiver<Messages>) -> bool {
    let stdout = child_process.stdout.take().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let mut stdout_lines = stdout_reader.lines();

    loop {
        let mut i = stdout_lines.next();
        while i.is_none() {                                     // blocking until new line is available=
            let possible_status = child_process.try_wait().unwrap();
            if !possible_status.is_none() {                     // if process completed
                let status = possible_status.unwrap();
                if status.success() {
                    return true;
                }
                match status.code() {
                    Some(code) => {println!("Exited with status code: {}", code); return false}
                    None       => {println!("Process terminated by signal"); return false}
                };
            }
            if let Ok(msg) = r.try_recv() {                     // If new message is available
                if msg == Messages::Terminate {
                    child_process.kill().expect("Command was not running.");
                    return false;
                }
            }
            i = stdout_lines.next();
        }
        info!(format!("[{}] Command ({}): {}", path, command, i.unwrap().unwrap()));
    }
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}

#[derive(PartialEq)]
enum Messages {
    // Test, // dev
    Terminate,
    // Terminated,
}
