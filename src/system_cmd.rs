use std::process::Command;
use failure::{Error, err_msg};
use regex::Regex;

use crate::model::project::branch::Branch;

fn run_system_command(command: &Vec<&str>, path: &str) -> Result<String, Error> {
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
    if raw_output.is_err() || !raw_output.unwrap().status.success() {
        return Err(err_msg(format!("Command failed ({})", command)));
    }
    
    Ok(String::from_utf8(raw_output.unwrap().stdout)?)
}

pub fn get_remote_git_repository_commits(remote_url: &str) -> Result<Vec<Branch>, Error> {
    let result = run_system_command(&["git", "ls-remote", "--heads", remote_url], "./")?;
    let regex_pattern = Regex::new(r"([0-9a-fA-F]+)\s+refs\/heads\/(\S+)").unwrap(); // Overkill, might change later
    let branches: Vec<Branch> = Vec::new();
    for capture in regex_pattern.captures_iter(result) {
        branches.push(Branch {
            name: capture.get(2).unwrap().as_str().to_string(),
            latest_commit_hash: capture.get(1).unwrap().as_str().to_string()
        })
    }

    Ok(branches)
}