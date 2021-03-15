use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Branch {
    pub name: String,
    pub latest_commit_hash: String,
}