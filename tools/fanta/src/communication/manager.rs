use std::collections::HashMap;

use log::{debug, error};
use serde_json::Value;
use tokio::sync::mpsc;

use super::{receiver::SubscribedOutput, Cycler, CyclerOutput};

#[derive(Debug)]
pub enum Message {
    SubscribeOutput {
        output: CyclerOutput,
        output_sender: mpsc::Sender<Value>,
    },
    OutputsUpdated {
        cycler: Cycler,
        outputs: Vec<SubscribedOutput>,
    },
}

pub async fn manager(mut receiver: mpsc::Receiver<Message>) {
    let mut subscribed_outputs = HashMap::new();
    while let Some(message) = receiver.recv().await {
        debug!("commander got message: {message:?}");
        match message {
            Message::SubscribeOutput {
                output,
                output_sender,
            } => {
                subscribed_outputs.insert(output, output_sender);
            }
            Message::OutputsUpdated { cycler, outputs } => {
                for output in outputs {
                    if let Some(sender) = subscribed_outputs.get(&CyclerOutput {
                        cycler,
                        output: output.output,
                    }) {
                        if let Err(error) = sender.send(output.data).await {
                            error!("Failed to send updated output to listener: {error:?}");
                        }
                    }
                }
            }
        }
    }
}
