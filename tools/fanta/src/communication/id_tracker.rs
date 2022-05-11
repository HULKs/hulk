use log::error;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum Message {
    GetId {
        response_sender: oneshot::Sender<usize>,
    },
}

pub async fn id_tracker(mut receiver: mpsc::Receiver<Message>) {
    let mut id = 0;
    while let Some(message) = receiver.recv().await {
        match message {
            Message::GetId { response_sender } => {
                if let Err(error) = response_sender.send(id) {
                    error!("Failed to send to response: {error:?}");
                    continue;
                }
                id = id.wrapping_add(1);
            }
        }
    }
}
