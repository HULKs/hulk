use communication::client::{Communication, Cycler, CyclerOutput, Output, SubscriberMessage};
use log::error;
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
        response_sender: oneshot::Sender<Result<Vec<u8>, String>>,
    },
    ListenToUpdates {
        response_sender: mpsc::Sender<()>,
    },
}

pub struct ImageBuffer {
    sender: mpsc::Sender<Message>,
}

impl ImageBuffer {
    pub fn new(communication: Communication, cycler: Cycler) -> Self {
        let output = CyclerOutput {
            cycler,
            output: Output::Main {
                path: "image".to_string(),
            },
        };
        let (command_sender, command_receiver) = mpsc::channel(10);
        spawn(async move {
            let (uuid, receiver) = communication
                .subscribe_output(output.clone(), communication::messages::Format::Binary)
                .await;
            image_buffer(receiver, command_receiver).await;
            communication.unsubscribe_output(uuid).await;
        });
        Self {
            sender: command_sender,
        }
    }

    #[allow(dead_code)]
    pub fn listen_to_updates(&self, response_sender: mpsc::Sender<()>) {
        self.sender
            .blocking_send(Message::ListenToUpdates { response_sender })
            .unwrap()
    }

    pub fn get_latest(&self) -> Result<Vec<u8>, String> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .blocking_send(Message::GetLatest {
                response_sender: sender,
            })
            .unwrap();
        receiver.blocking_recv().unwrap()
    }
}

async fn image_buffer(
    mut subscriber_receiver: mpsc::Receiver<SubscriberMessage>,
    mut command_receiver: mpsc::Receiver<Message>,
) {
    let mut image_data: Option<Result<Vec<u8>, String>> = None;
    let mut update_listeners: Vec<mpsc::Sender<()>> = Vec::new();
    loop {
        select! {
            maybe_message = subscriber_receiver.recv() => {
                match maybe_message {
                    Some(message) => {
                        match message {
                            SubscriberMessage::UpdateBinary{data: new_data} => {
                                image_data = Some(Ok(new_data));
                                update_listeners.retain(|listener| {
                                    if let Err(TrySendError::Closed(_)) = listener.try_send(()) {
                                            return false;
                                    }
                                    true
                                });
                            },
                            SubscriberMessage::SubscriptionSuccess => {},
                            SubscriberMessage::SubscriptionFailure{info} => {
                                image_data = Some(Err(info))
                            },
                            SubscriberMessage::Update{..} => {
                                error!("Got Update message on image buffer");
                                break;
                            }
                        }
                    },
                    None => break,
                }
            }
            maybe_command = command_receiver.recv() => {
                match maybe_command {
                    Some(command) => match command {
                        Message::GetLatest{response_sender} => {
                            let response = match &image_data {
                                Some(Ok(values)) => Ok(values.clone()),
                                Some(Err(error)) => Err(error.clone()),
                                None => Err("No response yet".to_string()),
                            };
                            response_sender.send(response).unwrap();
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
