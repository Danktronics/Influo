use std::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel, error::SendError};

pub mod message;

use message::Command;

#[derive(Debug)]
pub struct Channel<T> {
    pub receiver: Mutex<UnboundedReceiver<T>>,
    pub sender: UnboundedSender<T>,
}

#[derive(Debug)]
pub struct PipelineConnection {
    pub remote_url: String,
    pub branch_name: String,
    pub pipeline_name: String,
    sender: UnboundedSender<Command>
}

impl PipelineConnection {
    pub fn new(remote_url: String, branch_name: String, pipeline_name: String) -> (PipelineConnection, UnboundedReceiver<Command>) {
        let (sender, receiver) = unbounded_channel();
        (PipelineConnection {
            remote_url,
            branch_name,
            pipeline_name,
            sender
        }, receiver)
    }

    pub fn send(&self, command: Command) -> Result<(), SendError<Command>> {
        self.sender.send(command)
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}
