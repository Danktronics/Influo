use std::{
    fs,
    process::Stdio
};
//use tokio::io::{BufReader, AsyncBufReadExt};
use anyhow::{Error, anyhow};
use regex::Regex;

use crate::model::project::branch::Branch;

/// Synchronous function for running a system command in a child process
fn run_system_command(command: &str, path: &str) -> Result<String, Error> {
    let raw_output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
                .current_dir(path)
                .arg("/C")
                .args(&vec![command])
                .output()
    } else { // Assume Linux, BSD, and OSX
        std::process::Command::new("sh")
                .current_dir(path)
                .arg("-c") // Non-login and non-interactive
                .args(&vec![command])
                .output()
    };
    let output = raw_output?;
    if !output.status.success() {
        let human_exit_code = if output.status.code().is_some() {
            output.status.code().unwrap()
        } else {
            1 // Child process terminated by signal (UNIX) (should probably retrieve signal)
        };
        debug!(format!("System command failed ({}) with status: {}", command, human_exit_code));
        return Err(anyhow!("System command failure with code {}", human_exit_code));
    }

    Ok(String::from_utf8(output.stdout)?)
}

/// Retrieves the remote git branches synchronously using git ls-remote
pub fn get_remote_git_repository_commits(remote_url: &str) -> Result<Vec<Branch>, Error> {
    let result: String = run_system_command(&format!("git ls-remote --heads {}", remote_url), "./")?;
    let regex_pattern = Regex::new(r"([0-9a-fA-F]+)\s+refs/heads/(\S+)").unwrap();
    let mut branches: Vec<Branch> = Vec::new();
    for capture in regex_pattern.captures_iter(&result) {
        branches.push(Branch {
            name: capture.get(2).unwrap().as_str().to_string(),
            latest_commit_hash: capture.get(1).unwrap().as_str().to_string()
        });
    }

    Ok(branches)
}

pub fn setup_git_repository(remote_url: &str, project_deploy_path: &str, branch: &str) -> Result<String, Error> {
    // Download or update repository
    let regex_pattern = Regex::new(r"^(https|git)(://|@)([^/:]+)[/:]([^/:]+)/([^.]*)[.git]*?$").unwrap();
    let possible_captures = regex_pattern.captures(remote_url);
    if possible_captures.is_none() {
        error!(format!("Remote url ({}) did not pass regex", remote_url));
        return Err(anyhow!("Remote url ({}) did not pass regex", remote_url));
    }
    let captures = possible_captures.unwrap();
    let possible_repository_name = captures.get(captures.len() - 1);
    if possible_repository_name.is_none() {
        error!(format!("Remote url ({}) does not contain a valid name", remote_url));
        return Err(anyhow!("Remote url ({}) does not contain a valid name", remote_url));
    }
    let repository_name: &str = possible_repository_name.unwrap().as_str();
    let project_path: String = format!("{}/{}", project_deploy_path, repository_name);

    // Make sure the deploy path is valid
    fs::create_dir_all(&project_path)?;

    let clone_attempt = run_system_command(&format!("git clone --single-branch --branch {} {} {}", branch, remote_url, branch), &project_path);
    if clone_attempt.is_err() {
        if let Err(e0) = clone_attempt {
            debug!(format!("Git clone attempt failed for {} due to: {}", remote_url, e0));
            let pull_attempt = run_system_command(&format!("git pull origin {}", branch), &format!("{}/{}", project_path, branch));
            if pull_attempt.is_err() {
                if let Err(e1) = pull_attempt {
                    debug!(format!("Git pull attempt failed for {} due to: {}", remote_url, e1));
                    error!(format!("Failed to update/create git repository with URL: {} and branch: {} in path: {}", remote_url, branch, project_path));
                    return Err(anyhow!("Failed to update/create git repository with URL: {} and branch: {} in path: {}", remote_url, branch, project_path));
                }
            }
        }
    }

    Ok(repository_name.to_string())
}

/// Special system command runner for long running children
/// Procedure commands are not guaranteed to end
pub fn run_procedure_command(command: &str, repository_path: &str) -> Result<tokio::process::Child, Error> {
    if cfg!(target_os = "windows") {
        Ok(tokio::process::Command::new("cmd")
                .current_dir(repository_path)
                .arg("/C")
                .args(&vec![command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?)
    } else { // Assume Linux, BSD, and OSX
        let args = shell_words::split(command).unwrap();
        Ok(tokio::process::Command::new(&args[0])
                .current_dir(repository_path)
                //.arg("-c") // Non-login and non-interactive
                .args(if args.is_empty() { &[][..] } else { &args[1..] })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?)
    }
}
