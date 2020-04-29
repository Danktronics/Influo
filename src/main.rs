use std::fs;
use std::thread;
use std::time::Duration;
use failure::{Error, err_msg};
use serde_json::Value;

mod model;

use model::project::Project;

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;

    let update_interval: &Value = &config["update_interval"];
    if update_interval.is_null() || !update_interval.is_number() {
        setup_updater_thread(30);
    } else {
        let interval: Option<u64> = update_interval.as_u64();
        if interval.is_none() || interval.unwrap() > u32::MAX as u64 {
            panic!("The integer provided exceeded the u32 max");
        }
        let join_handle: thread::JoinHandle<()> = setup_updater_thread(interval.unwrap() as u32 * 1000);
        join_handle.join().unwrap();
    }

    let raw_projects: &Value = &config["projects"];
    if !raw_projects.is_array() {
        return Err(err_msg("Projects is invalid"));
    }
    let raw_projects_array: &Vec<Value> = raw_projects.as_array().unwrap();
    let projects: Vec<Project> = Vec::new();
    for raw_project in raw_projects_array {
        projects.push(Project::new(raw_project["url"], raw_project["procedures"])?.unwrap());
    }

    Ok(())
}

/// Interval should be in milliseconds
fn setup_updater_thread(interval: u32) -> thread::JoinHandle<()> {
    println!("Spawning updater thread");
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(interval as u64));
        println!("Run here")
    })
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}
