use std::collections::HashMap;

use anyhow::Result;
use log::{debug, error};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum Message {
    Wait {
        id: usize,
        response_sender: oneshot::Sender<Result<()>>,
    },
    Respond {
        id: usize,
        response: Result<()>,
    },
}

pub async fn responder(mut receiver: mpsc::Receiver<Message>) {
    let mut awaiting_response = HashMap::new();
    while let Some(message) = receiver.recv().await {
        debug!("Responder got message: {message:?}");
        match message {
            Message::Wait {
                id,
                response_sender,
            } => {
                awaiting_response.insert(id, response_sender);
            }
            Message::Respond { id, response } => match awaiting_response.remove(&id) {
                Some(sender) => {
                    if let Err(error) = sender.send(response) {
                        error!("Failed to send to response channel: {error:?}");
                    }
                }
                None => error!("Cannot find sender waiting for a response with id '{id}'"),
            },
        }
    }
}
