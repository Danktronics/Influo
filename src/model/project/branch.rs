use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub latest_commit_hash: String,
}