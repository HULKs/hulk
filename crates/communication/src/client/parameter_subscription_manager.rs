use std::collections::{hash_map::Entry, BTreeSet, HashMap};

use color_eyre::eyre::Result;
use log::{error, info, warn};
use serde_json::Value;
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

use crate::{
    client::{
        id_tracker::{self, get_message_id},
        responder, SubscriberMessage,
    },
    messages::{ParametersRequest, Path, Request},
};

use super::responder::Response;

#[derive(Debug)]
pub enum Message {
    Connect {
        requester: mpsc::Sender<Request>,
    },
    Disconnect,
    Subscribe {
        path: String,
        subscriber: mpsc::Sender<SubscriberMessage>,
        response_sender: oneshot::Sender<Uuid>,
    },
    Unsubscribe {
        uuid: Uuid,
    },
    Update {
        subscription_id: usize,
        data: Value,
    },
    UpdateFields {
        fields: BTreeSet<Path>,
    },
    GetFields {
        response_sender: oneshot::Sender<Option<BTreeSet<Path>>>,
    },
    UpdateParameterValue {
        path: String,
        value: Value,
    },
}

#[derive(Default)]
struct SubscriptionManager {
    ids_to_paths: HashMap<usize, Path>,
    paths_to_subscribers: HashMap<Path, HashMap<Uuid, mpsc::Sender<SubscriberMessage>>>,
}

pub async fn parameter_subscription_manager(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
    responder: mpsc::Sender<responder::Message>,
) {
    let mut manager = SubscriptionManager::default();
    let mut requester = None;
    let mut fields = None;
    while let Some(message) = receiver.recv().await {
        match message {
            Message::Connect {
                requester: new_requester,
            } => {
                assert!(manager.ids_to_paths.is_empty());
                manager.ids_to_paths.clear();
                for (path, subscribers) in &manager.paths_to_subscribers {
                    let subscribers = subscribers.values().cloned().collect();
                    if let Some(subscription_id) = subscribe(
                        path.clone(),
                        subscribers,
                        &id_tracker,
                        &responder,
                        &new_requester,
                    )
                    .await
                    {
                        manager.ids_to_paths.insert(subscription_id, path.clone());
                    }
                }
                query_parameter_hierarchy(sender.clone(), &id_tracker, &responder, &new_requester)
                    .await;
                requester = Some(new_requester);
            }
            Message::Disconnect => {
                requester = None;
                manager.ids_to_paths.clear();
            }
            Message::Subscribe {
                path,
                subscriber,
                response_sender,
            } => {
                let uuid = Uuid::new_v4();
                match response_sender.send(uuid) {
                    Ok(()) => {
                        add_subscription(
                            &mut manager,
                            uuid,
                            path,
                            subscriber,
                            &id_tracker,
                            &responder,
                            &requester,
                        )
                        .await
                    }
                    Err(error) => error!("{error}"),
                };
            }
            Message::Unsubscribe { uuid } => {
                let mut subscriptions_to_remove = Vec::new();
                manager.paths_to_subscribers.retain(|path, clients| {
                    if clients.remove(&uuid).is_none() {
                        return true;
                    }

                    if clients.is_empty() {
                        let maybe_subscription_id = manager
                            .ids_to_paths
                            .iter()
                            .find_map(|(id, other_path)| (path == other_path).then_some(*id));
                        if let Some(id) = maybe_subscription_id {
                            subscriptions_to_remove.push(id);
                        }
                    }
                    !clients.is_empty()
                });
                for subscription_id in subscriptions_to_remove {
                    if let Some(requester) = &requester {
                        manager.ids_to_paths.remove(&subscription_id);
                        unsubscribe(subscription_id, &id_tracker, &responder, requester).await;
                    }
                }
            }
            Message::Update {
                subscription_id,
                data,
            } => {
                let Some(path) = manager.ids_to_paths.get(&subscription_id) else {
                    return warn!("Unknown subscription_id: {subscription_id}");
                };
                let Some(senders) = manager.paths_to_subscribers.get(path) else {
                    return warn!("Unknown subscription_id: {subscription_id}");
                };
                for sender in senders.values() {
                    if let Err(error) = sender
                        .send(SubscriberMessage::Update {
                            value: data.clone(),
                        })
                        .await
                    {
                        error!("{error}");
                    }
                }
            }
            Message::UpdateFields { fields: new_fields } => {
                fields = Some(new_fields);
            }
            Message::GetFields { response_sender } => {
                if let Err(error) = response_sender.send(fields.clone()) {
                    error!("{error:?}");
                }
            }
            Message::UpdateParameterValue { path, value } => {
                if let Some(some_requester) = requester {
                    match update_parameter_value(
                        path,
                        value,
                        &id_tracker,
                        &responder,
                        &some_requester,
                    )
                    .await
                    {
                        Ok(_) => requester = Some(some_requester),
                        Err(error) => {
                            error!("{error}");
                            requester = None
                        }
                    }
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
    requester: &mpsc::Sender<Request>,
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
        .send(Request::Parameters(ParametersRequest::GetFields {
            id: message_id,
        }))
        .await
        .unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Response::ParameterFields(fields) => manager
                .send(Message::UpdateFields { fields })
                .await
                .unwrap(),
            response => error!("unexpected response: {response:?}"),
        }
    });
}

async fn update_parameter_value(
    path: String,
    value: Value,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<Request>,
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
        .send(Request::Parameters(ParametersRequest::Update {
            id: message_id,
            path,
            data: value,
        }))
        .await?;
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Response::Update(Ok(_)) => {}
            Response::Update(Err(error)) => {
                error!("Failed to update value: {}", error)
            }
            response => error!("unexpected response: {response:?}"),
        };
    });

    Ok(())
}

async fn add_subscription(
    manager: &mut SubscriptionManager,
    uuid: Uuid,
    path: String,
    subscriber: mpsc::Sender<SubscriberMessage>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &Option<mpsc::Sender<Request>>,
) {
    match manager.paths_to_subscribers.entry(path.clone()) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().insert(uuid, subscriber);
        }
        Entry::Vacant(entry) => {
            if let Some(requester) = requester {
                if let Some(subscription_id) = subscribe(
                    path.clone(),
                    vec![subscriber.clone()],
                    id_tracker,
                    responder,
                    requester,
                )
                .await
                {
                    manager.ids_to_paths.insert(subscription_id, path);
                }
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
    requester: &mpsc::Sender<Request>,
) -> Option<usize> {
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
        return None;
    }
    let request = Request::Parameters(ParametersRequest::Subscribe {
        id: message_id,
        path,
    });
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let message = match response {
            Response::Subscribe(Ok(_)) => SubscriberMessage::SubscriptionSuccess,
            Response::Subscribe(Err(error)) => {
                SubscriberMessage::SubscriptionFailure { info: error }
            }
            response => return error!("unexpected response: {response:?}"),
        };
        for sender in subscribers {
            if let Err(error) = sender.send(message.clone()).await {
                error!("{error}");
            }
        }
    });

    Some(message_id)
}

async fn unsubscribe(
    subscription_id: usize,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<Request>,
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
    let request = Request::Parameters(ParametersRequest::Unsubscribe {
        id: message_id,
        subscription_id,
    });
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Response::Unsubscribe(Ok(_)) => {}
            Response::Unsubscribe(Err(error)) => {
                error!("Failed to unsubscribe: {}", error)
            }
            response => error!("unexpected response: {response:?}"),
        };
    });
}
