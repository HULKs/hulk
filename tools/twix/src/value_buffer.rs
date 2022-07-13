use std::collections::VecDeque;

use anyhow::{anyhow, Context, Result};
use communication::{Communication, CyclerOutput, SubscriberMessage};
use serde::Deserialize;
use serde_json::{from_value, Value, Value::Array};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, error::TrySendError},
        oneshot,
    },
};

#[derive(Debug)]
enum Message {
    GetLatest {
        response_sender: oneshot::Sender<Result<Value, String>>,
    },
    GetBuffered {
        response_sender: oneshot::Sender<Result<Vec<Value>, String>>,
    },
    SetBufferSize {
        buffer_size: usize,
    },
    ListenToUpdates {
        response_sender: mpsc::Sender<()>,
    },
}

pub struct ValueBuffer {
    sender: mpsc::Sender<Message>,
}

impl ValueBuffer {
    pub fn output(communication: Communication, output: CyclerOutput) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication.subscribe_output(output.clone()).await;
            value_buffer(receiver, command_receiver).await;
            communication.unsubscribe_output(output, uuid).await;
        });
        Self {
            sender: command_sender,
        }
    }

    pub fn parameter(communication: Communication, path: String) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication.subscribe_parameter(path.clone()).await;
            value_buffer(receiver, command_receiver).await;
            communication.unsubscribe_parameter(path, uuid).await;
        });
        Self {
            sender: command_sender,
        }
    }

    pub fn listen_to_updates(&self, response_sender: mpsc::Sender<()>) {
        self.sender
            .blocking_send(Message::ListenToUpdates { response_sender })
            .unwrap()
    }

    pub fn get_latest(&self) -> Result<Value, String> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .blocking_send(Message::GetLatest {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn get_buffered(&self) -> Result<Vec<Value>, String> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .blocking_send(Message::GetBuffered {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn set_buffer_size(&self, buffer_size: usize) {
        self.sender
            .blocking_send(Message::SetBufferSize { buffer_size })
            .unwrap();
    }
    pub fn parse_latest<Output>(&self) -> Result<Output>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let latest_value = self.get_latest().map_err(|error| anyhow!(error))?;
        from_value(latest_value).context("Failed to parse json value")
    }

    pub fn require_latest<Output>(&self) -> Result<Output>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let parsed_value: Option<Output> = self.parse_latest()?;
        parsed_value.ok_or_else(|| anyhow!("Value was none"))
    }

    pub fn parse_buffered<Output>(&self) -> Result<Vec<Output>>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let buffered_values = self.get_buffered().map_err(|error| anyhow!(error))?;
        from_value(Array(buffered_values)).context("Failed to parse json value")
    }
}

async fn value_buffer(
    mut subscriber_receiver: mpsc::Receiver<SubscriberMessage>,
    mut command_receiver: mpsc::Receiver<Message>,
) {
    let mut values: Option<Result<VecDeque<Value>, String>> = None;
    let mut update_listeners: Vec<mpsc::Sender<()>> = Vec::new();
    let mut buffer_size = 1;
    loop {
        select! {
            maybe_message = subscriber_receiver.recv() => {
                match maybe_message {
                    Some(message) => {
                        match message {
                            SubscriberMessage::Update{value:new_value} => {
                                match &mut values {
                                    Some(Ok(values)) => {
                                        values.push_front(new_value);
                                        values.truncate(buffer_size);
                                    },
                                    _ => {
                                        let mut new_buffer = VecDeque::with_capacity(buffer_size);
                                        new_buffer.push_back(new_value);
                                        values = Some(Ok(new_buffer));
                                    },
                                }
                                update_listeners.retain(|listener| {
                                    if let Err(TrySendError::Closed(_)) = listener.try_send(()) {
                                            return false;
                                    }
                                    true
                                });
                            },
                            SubscriberMessage::SubscriptionSuccess => (),
                            SubscriberMessage::SubscriptionFailure{info} => values = Some(Err(info)),
                        }
                    },
                    None => break,
                }
            }
            maybe_command = command_receiver.recv() => {
                match maybe_command {
                    Some(command) => match command {
                        Message::GetLatest{response_sender} => {
                            let response = match &values {
                                Some(Ok(values)) => Ok(values.front().unwrap().clone()),
                                Some(Err(error)) => Err(error.clone()),
                                None => Err("No response yet".to_string()),
                            };
                            response_sender.send(response).unwrap();
                        },
                        Message::GetBuffered{response_sender} => {
                            let response = match &values {
                                Some(Ok(values)) => Ok(values.iter().cloned().collect()),
                                Some(Err(error)) => Err(error.clone()),
                                None => Err("No response yet".to_string()),
                            };
                            response_sender.send(response).unwrap();
                        },
                        Message::SetBufferSize{buffer_size:new_buffer_size} => {
                            buffer_size = new_buffer_size;
                            if let Some(Ok(values)) = &mut values {
                                values.truncate(buffer_size);
                            }
                        },
                        Message::ListenToUpdates{response_sender} => {
                            update_listeners.push(response_sender)
                        },
                    },
                    None => break,
                }
            }
        }
    }
}
