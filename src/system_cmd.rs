use std::process::Command;
use std::fs;
use std::process::{Child, Stdio};
use failure::{Error, err_msg};
use regex::Regex;

use crate::model::project::branch::Branch;

fn run_system_command(command: &Vec<&String>, path: &str) -> Result<String, Error> {
    let raw_output = if cfg!(target_os = "windows") {
        Command::new("cmd")
                .current_dir(path)
                .arg("/C")
                .args(command)
                .output()
    } else { // Assume Linux, BSD, and OSX
        Command::new("sh")
                .current_dir(path)
                .arg("-c") // Non-login and non-interactive
                .args(command)
                .output()
    };
    let output = raw_output?;
    if !output.status.success() {
        println!("{:?}", output);
        return Err(err_msg(format!("Command failed ({:?})", command)));
    }
    
    Ok(String::from_utf8(output.stdout)?)
}

pub fn get_remote_git_repository_commits(remote_url: &str) -> Result<Vec<Branch>, Error> {
    let mut command: String = String::from("git ls-remote ");
    command.push_str("--heads ");
    command.push_str(remote_url);
    let result = run_system_command(&vec![&command], "./")?;
    let regex_pattern = Regex::new(r"([0-9a-fA-F]+)\s+refs/heads/(\S+)").unwrap(); // Overkill, might change later
    let mut branches: Vec<Branch> = Vec::new();
    for capture in regex_pattern.captures_iter(&result) {
        branches.push(Branch {
            name: capture.get(2).unwrap().as_str().to_string(),
            latest_commit_hash: capture.get(1).unwrap().as_str().to_string()
        })
    }

    Ok(branches)
}

pub fn setup_git_repository(remote_url: &str, deploy_path: &str) -> Result<String, Error> {
    // Make sure the deploy path is valid
    fs::create_dir_all(deploy_path)?;

    // Download or update repository
    let regex_pattern = Regex::new(r"^(https|git)(:\/\/|@)([^\/:]+)[\/:]([^\/:]+)\/([^.]*)[.git]*?$").unwrap();
    let captures = regex_pattern.captures(remote_url)?;
    let repository_name = captures.get(captures.len());

    let clone_attempt = run_system_command(&vec!(&format!("git clone {}", remote_url)), deploy_path);
    if clone_attempt.is_err() {
        let pull_attempt = run_system_command(&vec!(&"git pull"), format!("{}/{}", deploy_path, repository_name));
        if pull_attempt.is_err() {
            return Err(err_msg("Failed to update repository (clone and pull failed)"));
        }
    }

    Ok(repository_name)
}

// Procedure commands are not guaranteed to end
pub fn run_procedure_command(command: &str, repository_path: &str) -> Result<Child, Error> {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
                .current_dir(repository_path)
                .arg("/C")
                .args(&vec!(command))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
    } else { // Assume Linux, BSD, and OSX
        Command::new("sh")
                .current_dir(repository_path)
                .arg("-c") // Non-login and non-interactive
                .args(&vec!(command))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
    }
}