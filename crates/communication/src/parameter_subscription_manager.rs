use std::collections::{hash_map::Entry, HashMap};

use log::{error, info};
use serde_json::Value;
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

use crate::{
    id_tracker::{self, get_message_id},
    requester, responder, HierarchyType, SubscriberMessage,
};

#[derive(Debug)]
pub enum Message {
    Connect {
        requester: mpsc::Sender<requester::Message>,
    },
    Disconnect,
    Subscribe {
        path: String,
        subscriber: mpsc::Sender<SubscriberMessage>,
        response_sender: oneshot::Sender<Uuid>,
    },
    Unsubscribe {
        path: String,
        uuid: Uuid,
    },
    Update {
        path: String,
        data: Value,
    },
    UpdateParameterHierarchy {
        hierarchy: HierarchyType,
    },
    GetParameterHierarchy {
        response_sender: oneshot::Sender<Option<HierarchyType>>,
    },
    UpdateParameterValue {
        path: String,
        value: Value,
    },
}

pub async fn parameter_subscription_manager(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
    responder: mpsc::Sender<responder::Message>,
) {
    let mut subscribed_parameters: HashMap<String, HashMap<Uuid, mpsc::Sender<SubscriberMessage>>> =
        HashMap::new();
    let mut requester = None;
    let mut hierarchy = None;
    while let Some(message) = receiver.recv().await {
        match message {
            Message::Connect {
                requester: new_requester,
            } => {
                for (path, subscribers) in &subscribed_parameters {
                    let subscribers = subscribers.values().cloned().collect();
                    subscribe(
                        path.clone(),
                        subscribers,
                        &id_tracker,
                        &responder,
                        &new_requester,
                    )
                    .await
                }
                query_parameter_hierarchy(sender.clone(), &id_tracker, &responder, &new_requester)
                    .await;
                requester = Some(new_requester);
            }
            Message::Disconnect => {
                requester = None;
            }
            Message::Subscribe {
                path,
                subscriber,
                response_sender,
            } => {
                let uuid = Uuid::new_v4();
                response_sender.send(uuid).unwrap();
                add_subscription(
                    &mut subscribed_parameters,
                    uuid,
                    path,
                    subscriber,
                    &id_tracker,
                    &responder,
                    &requester,
                )
                .await;
            }
            Message::Unsubscribe { path, uuid } => {
                let mut is_empty = false;
                if let Some(sender) = subscribed_parameters.get_mut(&path) {
                    sender.remove(&uuid);
                    is_empty = sender.is_empty();
                }
                if is_empty {
                    subscribed_parameters.remove(&path);
                    if let Some(requester) = &requester {
                        unsubscribe(path, &id_tracker, &responder, requester).await;
                    }
                }
            }
            Message::Update { path, data } => {
                if let Some(senders) = subscribed_parameters.get(&path) {
                    for sender in senders.values() {
                        sender
                            .send(SubscriberMessage::Update {
                                value: data.clone(),
                            })
                            .await
                            .unwrap()
                    }
                }
            }
            Message::UpdateParameterHierarchy {
                hierarchy: new_hierarchy,
            } => {
                hierarchy = Some(new_hierarchy);
            }
            Message::GetParameterHierarchy { response_sender } => {
                response_sender.send(hierarchy.clone()).unwrap();
            }
            Message::UpdateParameterValue { path, value } => {
                if let Some(requester) = &requester {
                    update_parameter_value(path, value, requester, &id_tracker, &responder).await;
                }
            }
        }
    }
    info!("Finished manager");
}

async fn query_parameter_hierarchy(
    manager: mpsc::Sender<Message>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    requester
        .send(requester::Message::GetParameterHierarchy { id: message_id })
        .await
        .unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Ok(value) => {
                let hierarchy = serde_json::from_value(value);
                match hierarchy {
                    Ok(hierarchy) => {
                        manager
                            .send(Message::UpdateParameterHierarchy { hierarchy })
                            .await
                            .unwrap();
                    }
                    Err(error) => error!("Failed to deserialize ParameterHierarchy: {}", error),
                }
            }
            Err(error) => error!("Failed to get parameter hierarchy: {}", error),
        }
    });
}

async fn update_parameter_value(
    path: String,
    value: Value,
    requester: &mpsc::Sender<requester::Message>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    requester
        .send(requester::Message::UpdateParameter {
            id: message_id,
            path,
            data: value,
        })
        .await
        .unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        if let Err(error) = response {
            error!("Failed to update parameter: {}", error)
        }
    });
}

async fn add_subscription(
    subscribed_parameters: &mut HashMap<String, HashMap<Uuid, mpsc::Sender<SubscriberMessage>>>,
    uuid: Uuid,
    path: String,
    subscriber: mpsc::Sender<SubscriberMessage>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &Option<mpsc::Sender<requester::Message>>,
) {
    match subscribed_parameters.entry(path.clone()) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().insert(uuid, subscriber);
        }
        Entry::Vacant(entry) => {
            if let Some(requester) = requester {
                subscribe(
                    path,
                    vec![subscriber.clone()],
                    id_tracker,
                    responder,
                    requester,
                )
                .await;
            };
            entry.insert(HashMap::new()).insert(uuid, subscriber);
        }
    };
}

async fn subscribe(
    path: String,
    subscribers: Vec<mpsc::Sender<SubscriberMessage>>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    let request = requester::Message::SubscribeParameter {
        id: message_id,
        path,
    };
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let message = match response {
            Ok(_) => SubscriberMessage::SubscriptionSuccess,
            Err(error) => SubscriberMessage::SubscriptionFailure { info: error },
        };
        for sender in subscribers {
            sender.send(message.clone()).await.unwrap();
        }
    });
}

async fn unsubscribe(
    path: String,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    let request = requester::Message::UnsubscribeParameter {
        id: message_id,
        path,
    };
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        if let Err(error) = response {
            error!("Failed to unsubscribe: {}", error)
        };
    });
}
