use std::sync::Arc;
use tokio::sync::oneshot::{Receiver, Sender, channel};

pub mod message;

use message::{Command, Response};

#[derive(Debug)]
pub struct Channel<T> {
    pub receiver: Receiver<T>,
    pub sender: Sender<T>,
}

#[derive(Debug)]
pub struct ThreadConnection {
    pub owner_channel: Channel<Command>, // Channel for the owner thread to send
    pub child_channel: Channel<Response>, // Channel for the child thread to send (spawned by owner)
}

impl ThreadConnection {
    pub fn new() -> ThreadConnection {
        let (owner_sender, owner_receiver) = channel();
        let (child_sender, child_receiver) = channel();
        ThreadConnection {
            owner_channel: Channel::<Command> {
                receiver: owner_receiver,
                sender: owner_sender,
            },
            child_channel: Channel::<Response> {
                receiver: child_receiver,
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
        let (owner_sender, owner_receiver) = channel();
        let (child_sender, child_receiver) = channel();
        ThreadProcedureConnection {
            remote_url: remote_url,
            branch: branch,
            procedure_name: procedure_name,
            owner_channel: Channel::<Command> {
                receiver: owner_receiver,
                sender: owner_sender,
            },
            child_channel: Channel::<Response> {
                receiver: child_receiver,
                sender: child_sender,
            }
        }
    }
}
