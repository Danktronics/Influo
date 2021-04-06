use std::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub mod message;

use message::Command;

#[derive(Debug)]
pub struct Channel<T> {
    pub receiver: Mutex<UnboundedReceiver<T>>,
    pub sender: UnboundedSender<T>,
}

#[derive(Debug)]
pub struct ProcedureConnection {
    pub remote_url: String,
    pub branch: String,
    pub procedure_name: String,
    pub sender: UnboundedSender<Command>, // Channel for the owner thread to send
}

impl ProcedureConnection {
    pub fn new(remote_url: String, branch: String, procedure_name: String) -> (ProcedureConnection, UnboundedReceiver<Command>) {
        let (sender, receiver) = unbounded_channel();
        (ProcedureConnection {
            remote_url,
            branch,
            procedure_name,
            sender
        }, receiver)
    }
}
