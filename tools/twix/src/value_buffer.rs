use std::collections::VecDeque;

use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use communication::{
    client::{Communication, SubscriberMessage},
    messages::{Format, Path},
};
use log::error;
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
    GetSize {
        response_sender: oneshot::Sender<Result<usize, String>>,
    },
    SetCapacity {
        buffer_capacity: usize,
    },
    ListenToUpdates {
        response_sender: mpsc::Sender<()>,
    },
    UpdateParameterValue {
        value: Value,
    },
}

pub struct ValueBuffer {
    command_sender: mpsc::Sender<Message>,
}

impl ValueBuffer {
    pub fn output(communication: Communication, path: Path) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication
                .subscribe_output(path.clone(), Format::Textual)
                .await;
            value_buffer(receiver, command_receiver, communication.clone(), None).await;
            communication.unsubscribe_output(uuid).await;
        });
        Self { command_sender }
    }

    pub fn parameter(communication: Communication, path: String) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication.subscribe_parameter(path.clone()).await;
            value_buffer(
                receiver,
                command_receiver,
                communication.clone(),
                Some(path),
            )
            .await;
            communication.unsubscribe_parameter(uuid).await;
        });
        Self { command_sender }
    }

    pub fn listen_to_updates(&self, response_sender: mpsc::Sender<()>) {
        self.command_sender
            .blocking_send(Message::ListenToUpdates { response_sender })
            .unwrap()
    }

    pub fn get_latest(&self) -> Result<Value, String> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .blocking_send(Message::GetLatest {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn get_buffered(&self) -> Result<Vec<Value>, String> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .blocking_send(Message::GetBuffered {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn reserve(&self, buffer_size: usize) {
        self.command_sender
            .blocking_send(Message::SetCapacity {
                buffer_capacity: buffer_size,
            })
            .unwrap();
    }

    pub fn size(&self) -> Result<usize, String> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .blocking_send(Message::GetSize {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn update_parameter_value(&self, value: Value) {
        self.command_sender
            .blocking_send(Message::UpdateParameterValue { value })
            .unwrap();
    }

    pub fn parse_latest<Output>(&self) -> Result<Output>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let latest_value = self.get_latest().map_err(|error| eyre!(error))?;
        from_value(latest_value).wrap_err("Failed to parse json value")
    }

    pub fn require_latest<Output>(&self) -> Result<Output>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let parsed_value: Option<Output> = self.parse_latest()?;
        parsed_value.ok_or_else(|| eyre!("Value was none"))
    }

    pub fn parse_buffered<Output>(&self) -> Result<Vec<Output>>
    where
        for<'de> Output: Deserialize<'de>,
    {
        let buffered_values = self.get_buffered().map_err(|error| eyre!(error))?;
        from_value(Array(buffered_values)).wrap_err("Failed to parse json value")
    }
}

async fn value_buffer(
    mut subscriber_receiver: mpsc::Receiver<SubscriberMessage>,
    mut command_receiver: mpsc::Receiver<Message>,
    communication: Communication,
    parameter_path: Option<String>,
) {
    let mut values: Option<Result<VecDeque<Value>, String>> = None;
    let mut update_listeners: Vec<mpsc::Sender<()>> = Vec::new();
    let mut buffer_capacity = 1;
    let mut skip_updates = 0;

    loop {
        select! {
            maybe_message = subscriber_receiver.recv() => {
                match maybe_message {
                    Some(message) => {
                        match message {
                            SubscriberMessage::Update{value} => {
                                if skip_updates > 0 {
                                    skip_updates -= 1;
                                    continue;
                                }
                                add_element(&mut values, buffer_capacity, value);
                                update_listeners.retain(|listener| {
                                    if let Err(TrySendError::Closed(_)) = listener.try_send(()) {
                                            return false;
                                    }
                                    true
                                });
                            },
                            SubscriberMessage::SubscriptionSuccess => (),
                            SubscriberMessage::SubscriptionFailure{info} => values = Some(Err(info)),
                            SubscriberMessage::UpdateBinary{..} => {
                                error!("Got UpdateBinary message in value buffer");
                                break;
                            }
                        }
                    },
                    None => continue,
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
                        Message::GetSize{response_sender} => {
                            let response = match &values {
                                Some(Ok(values)) => Ok(values.len()),
                                Some(Err(error)) => Err(error.clone()),
                                None => Err("No response yet".to_string())
                            };
                            response_sender.send(response).unwrap();
                        }
                        Message::SetCapacity{buffer_capacity:new_buffer_capacity} => {
                            buffer_capacity = new_buffer_capacity;
                            if let Some(Ok(values)) = &mut values {
                                values.truncate(buffer_capacity);
                            }
                        },
                        Message::ListenToUpdates{response_sender} => {
                            update_listeners.push(response_sender)
                        },
                        Message::UpdateParameterValue{value} => {
                            skip_updates += 1;
                            add_element(&mut values, buffer_capacity, value.clone());
                            communication.update_parameter_value(
                                parameter_path.as_ref().expect(
                                    "tried updating parameter on output value buffer"
                                ),
                                value,
                            ).await;
                        },
                    },
                    None => break,
                }
            }
        }
    }
}

fn add_element(
    values: &mut Option<Result<VecDeque<Value>, String>>,
    capacity: usize,
    value: Value,
) {
    match values {
        Some(Ok(values)) => {
            values.push_front(value);
            values.truncate(capacity);
        }
        _ => {
            let mut new_buffer = VecDeque::with_capacity(capacity);
            new_buffer.push_back(value);
            *values = Some(Ok(new_buffer));
        }
    }
}
