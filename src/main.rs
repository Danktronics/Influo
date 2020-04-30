use std::fs;
use std::thread;
use std::time::Duration;
use failure::{Error, err_msg};
use serde_json::Value;

mod model;
mod system_cmd;

use model::project::Project;
use system_cmd::get_remote_git_repository_commits;

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;

    let raw_projects: &Value = &config["projects"];
    if !raw_projects.is_array() {
        return Err(err_msg("Projects is invalid"));
    }
    let raw_projects_array: &Vec<Value> = raw_projects.as_array().unwrap();
    let mut projects: Vec<Project> = Vec::new();
    for raw_project in raw_projects_array {
        projects.push(Project::new(&raw_project["url"], &raw_project["procedures"])?);
    }

    let update_interval: &Value = &config["update_interval"];
    if update_interval.is_null() || !update_interval.is_number() {
        setup_updater_thread(30, &projects);
    } else {
        let interval: Option<u64> = update_interval.as_u64();
        if interval.is_none() || interval.unwrap() > u32::MAX as u64 {
            panic!("The integer provided exceeded the u32 max");
        }
        let join_handle: thread::JoinHandle<()> = setup_updater_thread(interval.unwrap() as u32 * 1000, &projects);
        join_handle.join().unwrap();
    }

    Ok(())
}

/// Interval should be in milliseconds
fn setup_updater_thread(interval: u32, projects: &'static Vec<Project>) -> thread::JoinHandle<()> {
    println!("Spawning updater thread");
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(interval as u64));
        println!("Checking project repositories for updates");
        for project in projects {
            let query_result = get_remote_git_repository_commits(&project.url);
            if query_result.is_err() {
                println!("Failed to query commits for project with url {}", project.url);
                continue;
            }

            let branches = query_result.unwrap();
            for branch in branches {
                let cached_branch = project.branches.iter().find(|&&b| b.name == branch.name);
                if cached_branch.is_some() && cached_branch.unwrap().latest_commit_hash == branch.latest_commit_hash {
                    continue;
                }

                println!("Branch change detected (new commit or new branch)");
            }

            project.update_branches(branches);
        }
    })
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}
