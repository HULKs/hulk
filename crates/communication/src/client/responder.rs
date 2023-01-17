use std::collections::{BTreeSet, HashMap};

use log::{debug, error};
use tokio::sync::{mpsc, oneshot};

use crate::messages::{Fields, Path, Reason};

#[derive(Debug)]
pub enum Message {
    Await {
        id: usize,
        response_sender: oneshot::Sender<Response>,
    },
    Respond {
        id: usize,
        response: Response,
    },
}

#[derive(Debug)]
pub enum Response {
    Fields(Fields),
    ParameterFields(BTreeSet<Path>),
    Subscribe(Result<(), Reason>),
    Unsubscribe(Result<(), Reason>),
}

pub async fn responder(mut receiver: mpsc::Receiver<Message>) {
    let mut awaiting_response = HashMap::new();
    while let Some(message) = receiver.recv().await {
        debug!("Responder got message: {message:?}");
        match message {
            Message::Await {
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
