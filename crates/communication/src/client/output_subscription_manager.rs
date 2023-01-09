use std::collections::{hash_map::Entry, HashMap};

use color_eyre::Result;
use log::{error, info};
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

use crate::client::{
    id_tracker::{self, get_message_id},
    requester, responder,
    types::SubscribedOutput,
    Output, OutputHierarchy, SubscriberMessage,
};

use super::{Cycler, CyclerOutput};

#[derive(Debug)]
pub enum Message {
    Connect {
        requester: mpsc::Sender<requester::Message>,
    },
    Disconnect,
    Subscribe {
        output: CyclerOutput,
        subscriber: mpsc::Sender<SubscriberMessage>,
        response_sender: oneshot::Sender<Uuid>,
    },
    Unsubscribe {
        output: CyclerOutput,
        uuid: Uuid,
    },
    Update {
        cycler: Cycler,
        outputs: Vec<SubscribedOutput>,
        image_id: Option<u32>,
    },
    UpdateImage {
        image_id: u32,
        data: Vec<u8>,
    },
    UpdateOutputHierarchy {
        hierarchy: OutputHierarchy,
    },
    GetOutputHierarchy {
        response_sender: oneshot::Sender<Option<OutputHierarchy>>,
    },
}

pub async fn output_subscription_manager(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
    responder: mpsc::Sender<responder::Message>,
) {
    let mut subscribed_outputs: HashMap<
        CyclerOutput,
        HashMap<Uuid, mpsc::Sender<SubscriberMessage>>,
    > = HashMap::new();
    let mut requester = None;
    let mut hierarchy = None;
    let mut images: HashMap<u32, Vec<u8>> = HashMap::new();
    let mut image_ids_waiting_for_image: HashMap<u32, Cycler> = HashMap::new();

    while let Some(message) = receiver.recv().await {
        match message {
            Message::Connect {
                requester: new_requester,
            } => {
                for (output, subscribers) in &subscribed_outputs {
                    let subscribers = subscribers.values().cloned().collect();
                    subscribe(
                        output.clone(),
                        subscribers,
                        &id_tracker,
                        &responder,
                        &new_requester,
                    )
                    .await
                }
                match query_output_hierarchy(
                    sender.clone(),
                    &id_tracker,
                    &responder,
                    &new_requester,
                )
                .await
                {
                    Ok(()) => requester = Some(new_requester),
                    Err(error) => {
                        error!("{error}");
                    }
                };
            }
            Message::Disconnect => {
                requester = None;
            }
            Message::Subscribe {
                output,
                subscriber: output_sender,
                response_sender,
            } => {
                let uuid = Uuid::new_v4();
                match response_sender.send(uuid) {
                    Ok(()) => {
                        add_subscription(
                            &mut subscribed_outputs,
                            uuid,
                            output,
                            output_sender,
                            &id_tracker,
                            &responder,
                            &requester,
                        )
                        .await
                    }
                    Err(error) => error!("{error}"),
                };
            }
            Message::Unsubscribe { output, uuid } => {
                let mut is_empty = false;
                if let Some(sender) = subscribed_outputs.get_mut(&output) {
                    sender.remove(&uuid);
                    is_empty = sender.is_empty();
                }
                if is_empty {
                    subscribed_outputs.remove(&output);
                    if let Some(requester) = &requester {
                        unsubscribe(output, &id_tracker, &responder, requester).await;
                    }
                }
            }
            Message::Update {
                cycler,
                outputs,
                image_id,
            } => {
                let image_subscribers = subscribed_outputs.get(&CyclerOutput {
                    cycler,
                    output: Output::Image,
                });
                if let (Some(image_id), Some(senders)) = (image_id, image_subscribers) {
                    match images.remove(&image_id) {
                        Some(image) => {
                            for sender in senders.values() {
                                if let Err(error) = sender
                                    .send(SubscriberMessage::UpdateImage {
                                        data: image.clone(),
                                    })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
                        }
                        None => {
                            image_ids_waiting_for_image.insert(image_id, cycler);
                        }
                    }
                }
                for output in outputs {
                    if let Some(senders) = subscribed_outputs.get(&CyclerOutput {
                        cycler,
                        output: output.output,
                    }) {
                        for sender in senders.values() {
                            if let Err(error) = sender
                                .send(SubscriberMessage::Update {
                                    value: output.data.clone(),
                                })
                                .await
                            {
                                error!("{error}");
                            }
                        }
                    }
                }
            }
            Message::UpdateOutputHierarchy {
                hierarchy: new_hierarchy,
            } => {
                hierarchy = Some(new_hierarchy);
            }
            Message::GetOutputHierarchy { response_sender } => {
                if let Err(error) = response_sender.send(hierarchy.clone()) {
                    error!("{error:?}");
                }
            }
            Message::UpdateImage { image_id, data } => {
                if let Some(cycler) = image_ids_waiting_for_image.get(&image_id) {
                    let image_subscribers = subscribed_outputs.get(&CyclerOutput {
                        cycler: *cycler,
                        output: Output::Image,
                    });
                    if let Some(senders) = image_subscribers {
                        for sender in senders.values() {
                            if let Err(error) = sender
                                .send(SubscriberMessage::UpdateImage { data: data.clone() })
                                .await
                            {
                                error!("{error}");
                            }
                        }
                    }
                } else {
                    images.insert(image_id, data);
                }
            }
        }
    }
    info!("Finished manager");
}

async fn query_output_hierarchy(
    manager: mpsc::Sender<Message>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) -> Result<()> {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await?;
    requester
        .send(requester::Message::GetOutputHierarchy { id: message_id })
        .await?;
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Ok(value) => {
                let hierarchy = serde_json::from_value(value);
                match hierarchy {
                    Ok(hierarchy) => {
                        if let Err(error) = manager
                            .send(Message::UpdateOutputHierarchy { hierarchy })
                            .await
                        {
                            error!("{error}");
                        };
                    }
                    Err(error) => error!("Failed to deserialize OutputHierarchy: {}", error),
                }
            }
            Err(error) => error!("Failed to get output hierarchy: {}", error),
        }
    });
    Ok(())
}

async fn add_subscription(
    subscribed_outputs: &mut HashMap<CyclerOutput, HashMap<Uuid, mpsc::Sender<SubscriberMessage>>>,
    uuid: Uuid,
    output: CyclerOutput,
    output_sender: mpsc::Sender<SubscriberMessage>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &Option<mpsc::Sender<requester::Message>>,
) {
    match subscribed_outputs.entry(output.clone()) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().insert(uuid, output_sender);
        }
        Entry::Vacant(entry) => {
            if let Some(requester) = requester {
                subscribe(
                    output,
                    vec![output_sender.clone()],
                    id_tracker,
                    responder,
                    requester,
                )
                .await;
            };
            entry.insert(HashMap::new()).insert(uuid, output_sender);
        }
    };
}

async fn subscribe(
    output: CyclerOutput,
    subscribers: Vec<mpsc::Sender<SubscriberMessage>>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    if let Err(error) = responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
    {
        error!("{error}");
        return;
    }
    let request = requester::Message::SubscribeOutput {
        id: message_id,
        output,
    };
    if let Err(error) = requester.send(request).await {
        error!("{error}");
        return;
    }
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let message = match response {
            Ok(_) => SubscriberMessage::SubscriptionSuccess,
            Err(error) => SubscriberMessage::SubscriptionFailure { info: error },
        };
        for sender in subscribers {
            if let Err(error) = sender.send(message.clone()).await {
                error!("{error}");
            }
        }
    });
}

async fn unsubscribe(
    output: CyclerOutput,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    if let Err(error) = responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
    {
        error!("{error}")
    }
    let request = requester::Message::UnsubscribeOutput {
        id: message_id,
        output,
    };
    if let Err(error) = requester.send(request).await {
        error!("{error}")
    }
    spawn(async move {
        let response = response_receiver.await.unwrap();
        if let Err(error) = response {
            error!("Failed to unsubscribe: {}", error)
        };
    });
}
