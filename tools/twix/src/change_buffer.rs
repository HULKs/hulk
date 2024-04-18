use communication::{
    client::{Communication, CyclerOutput, SubscriberMessage},
    messages::Format,
};
use log::error;
use serde_json::Value;
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, Receiver},
        oneshot,
    },
};

#[derive(Debug, Clone)]
pub struct Change {
    pub message_number: usize,
    pub value: Value,
}

#[derive(Debug)]
pub struct ChangeBufferUpdate {
    pub updates: Vec<Change>,
    pub message_count: usize,
}

pub enum Message {
    GetBuffered {
        response_sender: oneshot::Sender<Result<ChangeBufferUpdate, String>>,
    },
    Clear,
    Reset,
}

pub struct ChangeBuffer {
    command_sender: mpsc::Sender<Message>,
}

async fn change_buffer(
    mut subscriber_receiver: Receiver<SubscriberMessage>,
    mut command_receiver: Receiver<Message>,
) {
    let mut last_value: Option<Value> = None;
    let mut changes = Ok(Vec::<Change>::new());
    let mut message_count: usize = 0;

    loop {
        select! {
            maybe_message = subscriber_receiver.recv() => {
                match maybe_message {
                    Some(message) => {
                        match message {
                            SubscriberMessage::Update{value} => {
                                if !last_value.as_ref().is_some_and(|last_value|*last_value==value){
                                    last_value = Some(value.clone());
                                    add_change(&mut changes, Change { message_number: message_count, value });
                                }
                            },
                            SubscriberMessage::SubscriptionSuccess => (),
                            SubscriberMessage::SubscriptionFailure{info} => {
                                last_value = None;
                                changes = Err(info);
                            },
                            SubscriberMessage::UpdateBinary{..} => {
                                error!("Got UpdateBinary message in change buffer");
                                break;
                            }
                        }
                        message_count += 1;
                    },
                    None => continue,
                }
            }
            maybe_command = command_receiver.recv() => {
                match maybe_command {
                    Some(command) => match command {
                        Message::GetBuffered { response_sender } => {
                            match changes.as_mut() {
                                Ok(changes) => {
                                    let updates = changes.to_vec();
                                    response_sender.send(Ok(ChangeBufferUpdate{updates, message_count})).unwrap();
                                }
                                Err(error) => {
                                    response_sender.send(Err(error.clone())).unwrap();
                                }
                            }
                        },
                        Message::Clear => {
                            if let Ok(changes) = changes.as_mut() {
                                let last_change = changes.pop();

                                changes.clear();
                                message_count = 0;

                                if let Some(last_change) = last_change {
                                    changes.push(Change {
                                        message_number: 0,
                                        value: last_change.value,
                                    });
                                }
                            }
                        }
                        Message::Reset  => {
                            match changes.as_mut() {
                                Ok(changes) => changes.clear(),
                                Err(_) => {changes = Ok(Vec::new())}
                            }
                            message_count = 0;
                        }
                    },
                    None => break,
                }
            }
        }
    }
}

fn add_change(changes: &mut Result<Vec<Change>, String>, change: Change) {
    match changes {
        Ok(changes) => {
            changes.push(change);
        }
        Err(_) => {
            *changes = Ok(vec![change]);
        }
    }
}

impl ChangeBuffer {
    pub fn output(communication: Communication, output: CyclerOutput) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication
                .subscribe_output(output, Format::Textual)
                .await;

            change_buffer(receiver, command_receiver).await;
            communication.unsubscribe_output(uuid).await;
        });
        Self { command_sender }
    }

    pub fn get_buffered(&self) -> Result<ChangeBufferUpdate, String> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .blocking_send(Message::GetBuffered {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }

    pub fn clear(&self) {
        self.command_sender.blocking_send(Message::Clear).unwrap();
    }

    pub fn reset(&self) {
        self.command_sender.blocking_send(Message::Reset).unwrap();
    }
}
