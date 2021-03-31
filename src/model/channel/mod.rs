use std::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub mod message;

use message::{Command, Response};

#[derive(Debug)]
pub struct Channel<T> {
    pub receiver: Mutex<UnboundedReceiver<T>>,
    pub sender: UnboundedSender<T>,
}

#[derive(Debug)]
pub struct ThreadConnection {
    pub owner_channel: Channel<Command>, // Channel for the owner thread to send
    pub child_channel: Channel<Response>, // Channel for the child thread to send (spawned by owner)
}

impl ThreadConnection {
    pub fn new() -> ThreadConnection {
        let (owner_sender, owner_receiver) = unbounded_channel();
        let (child_sender, child_receiver) = unbounded_channel();
        ThreadConnection {
            owner_channel: Channel::<Command> {
                receiver: Mutex::new(owner_receiver),
                sender: owner_sender,
            },
            child_channel: Channel::<Response> {
                receiver: Mutex::new(child_receiver),
                sender: child_sender,
            }
        }
    }
}

#[derive(Debug)]
pub struct ThreadProcedureConnection {
    pub remote_url: String,
    pub branch: String,
    pub procedure_name: String,
    pub owner_channel: Channel<Command>, // Channel for the owner thread to send
    pub child_channel: Channel<Response>, // Channel for the child thread to send (spawned by owner)
}

impl ThreadProcedureConnection {
    pub fn new(remote_url: String, branch: String, procedure_name: String) -> ThreadProcedureConnection {
        let (owner_sender, owner_receiver) = unbounded_channel();
        let (child_sender, child_receiver) = unbounded_channel();
        ThreadProcedureConnection {
            remote_url,
            branch,
            procedure_name,
            owner_channel: Channel::<Command> {
                receiver: Mutex::new(owner_receiver),
                sender: owner_sender,
            },
            child_channel: Channel::<Response> {
                receiver: Mutex::new(child_receiver),
                sender: child_sender,
            }
        }
    }
}
