use std::collections::{hash_map::Entry, HashMap};

use color_eyre::Result;
use log::{error, info, warn};
use tokio::{
    spawn,
    sync::{broadcast, mpsc, oneshot},
};
use uuid::Uuid;

use crate::{
    client::{
        id_tracker::{self, get_message_id},
        responder, SubscriberMessage,
    },
    messages::{
        Fields, Format, OutputsRequest, Path, Request, TextualDataOrBinaryReference::{self, BinaryReference, TextualData}
    },
};

use super::responder::Response;

#[derive(Debug)]
pub enum Message {
    Connect {
        requester: mpsc::Sender<Request>,
    },
    Disconnect,
    Subscribe {
        path: Path,
        format: Format,
        subscriber: mpsc::Sender<SubscriberMessage>,
        response_sender: oneshot::Sender<Uuid>,
    },
    Unsubscribe {
        uuid: Uuid,
    },
    Update {
        items: HashMap<usize, TextualDataOrBinaryReference>,
    },
    UpdateBinary {
        referenced_items: HashMap<usize, Vec<u8>>,
    },
    UpdateFields {
        fields: Fields,
    },
    GetOutputFields {
        response_sender: oneshot::Sender<Option<Fields>>,
    },
}

#[derive(Default)]
struct SubscriptionManager {
    ids_to_outputs: HashMap<usize, (Path, Format)>,
    outputs_to_subscribers:
        HashMap<(Path, Format), HashMap<Uuid, mpsc::Sender<SubscriberMessage>>>,
}

pub async fn output_subscription_manager(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
    responder: mpsc::Sender<responder::Message>,
    update_sender: broadcast::Sender<()>,
) {
    let mut manager = SubscriptionManager::default();
    let mut requester = None;
    let mut fields = None;
    let mut binary_data_waiting_for_references: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut binary_references_waiting_for_data: HashMap<usize, Path> = HashMap::new();

    while let Some(message) = receiver.recv().await {
        match message {
            Message::Connect {
                requester: new_requester,
            } => {
                assert!(manager.ids_to_outputs.is_empty());
                for ((output, format), subscribers) in &manager.outputs_to_subscribers {
                    let subscribers = subscribers.values().cloned().collect();
                    if let Some(subscription_id) = subscribe(
                        output.clone(),
                        *format,
                        subscribers,
                        &id_tracker,
                        &responder,
                        &new_requester,
                    )
                    .await
                    {
                        manager
                            .ids_to_outputs
                            .insert(subscription_id, (output.clone(), *format));
                    }
                }
                match query_output_fields(sender.clone(), &id_tracker, &responder, &new_requester)
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
                manager.ids_to_outputs.clear();
            }
            Message::Subscribe {
                path,
                format,
                subscriber: output_sender,
                response_sender,
            } => {
                let uuid = Uuid::new_v4();
                match response_sender.send(uuid) {
                    Ok(()) => {
                        add_subscription(
                            &mut manager,
                            uuid,
                            path,
                            format,
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
            Message::Unsubscribe { uuid } => {
                let mut subscriptions_to_remove = Vec::new();
                manager
                    .outputs_to_subscribers
                    .retain(|output_format, clients| {
                        if clients.remove(&uuid).is_none() {
                            return true;
                        }

                        if clients.is_empty() {
                            let maybe_subscription_id =
                                manager
                                    .ids_to_outputs
                                    .iter()
                                    .find_map(|(id, other_output)| {
                                        (output_format == other_output).then_some(*id)
                                    });
                            if let Some(id) = maybe_subscription_id {
                                subscriptions_to_remove.push(id);
                            }
                        }
                        !clients.is_empty()
                    });
                for subscription_id in subscriptions_to_remove {
                    if let Some(requester) = &requester {
                        manager.ids_to_outputs.remove(&subscription_id);
                        unsubscribe(subscription_id, &id_tracker, &responder, requester).await;
                    }
                }
            }
            Message::Update { items } => {
                for (subscription_id, value_or_reference) in items {
                    let Some(output) = manager.ids_to_outputs.get(&subscription_id) else {
                        warn!("unknown subscription_id: {subscription_id}");
                        continue;
                    };
                    if let Some(senders) = manager.outputs_to_subscribers.get(output) {
                        match value_or_reference {
                            TextualData { data } => {
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
                            BinaryReference { reference_id } => {
                                if let Some(image) =
                                    binary_data_waiting_for_references.remove(&reference_id)
                                {
                                    for sender in senders.values() {
                                        if let Err(error) = sender
                                            .send(SubscriberMessage::UpdateBinary {
                                                data: image.clone(),
                                            })
                                            .await
                                        {
                                            error!("{error}");
                                        }
                                    }
                                } else {
                                    binary_references_waiting_for_data
                                        .insert(reference_id, output.0.clone());
                                }
                            }
                        }
                    }
                }
                let _ = update_sender.send(());
            }
            Message::UpdateFields { fields: new_fields } => {
                fields = Some(new_fields);
                let _ = update_sender.send(());
            }
            Message::GetOutputFields { response_sender } => {
                if let Err(error) = response_sender.send(fields.clone()) {
                    error!("{error:?}");
                }
            }
            Message::UpdateBinary { referenced_items } => {
                for (reference_id, data) in referenced_items {
                    if let Some(output) = binary_references_waiting_for_data.get(&reference_id) {
                        let subscribers = manager
                            .outputs_to_subscribers
                            .get(&(output.clone(), Format::Binary));
                        if let Some(senders) = subscribers {
                            for sender in senders.values() {
                                if let Err(error) = sender
                                    .send(SubscriberMessage::UpdateBinary { data: data.clone() })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
                        }
                    } else {
                        binary_data_waiting_for_references.insert(reference_id, data);
                    }
                }
                let _ = update_sender.send(());
            }
        }
    }
    info!("Finished manager");
}

async fn query_output_fields(
    manager: mpsc::Sender<Message>,
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
    let request = Request::Outputs(OutputsRequest::GetFields { id: message_id });
    requester.send(request).await?;
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Response::Fields(fields) => {
                if let Err(error) = manager.send(Message::UpdateFields { fields }).await {
                    error!("{error}");
                };
            }
            response => error!("unexpected response: {response:?}"),
        }
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_subscription(
    manager: &mut SubscriptionManager,
    uuid: Uuid,
    path: Path,
    format: Format,
    output_sender: mpsc::Sender<SubscriberMessage>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &Option<mpsc::Sender<Request>>,
) {
    match manager
        .outputs_to_subscribers
        .entry((path.clone(), format))
    {
        Entry::Occupied(mut entry) => {
            entry.get_mut().insert(uuid, output_sender);
        }
        Entry::Vacant(entry) => {
            if let Some(requester) = requester {
                if let Some(subscription_id) = subscribe(
                    path.clone(),
                    format,
                    vec![output_sender.clone()],
                    id_tracker,
                    responder,
                    requester,
                )
                .await
                {
                    manager
                        .ids_to_outputs
                        .insert(subscription_id, (path, format));
                }
            };
            entry.insert(HashMap::new()).insert(uuid, output_sender);
        }
    };
}

async fn subscribe(
    path: Path,
    format: Format,
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
    let request = Request::Outputs(OutputsRequest::Subscribe {
        id: message_id,
        path,
        format,
    });
    if let Err(error) = requester.send(request).await {
        error!("{error}");
        return None;
    }
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let result = match response {
            Response::Subscribe(result) => result,
            response => return error!("unexpected response: {response:?}"),
        };
        let message = match result {
            Ok(()) => SubscriberMessage::SubscriptionSuccess,
            Err(error) => SubscriberMessage::SubscriptionFailure { info: error },
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
    if let Err(error) = responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
    {
        error!("{error}")
    }
    let request = Request::Outputs(OutputsRequest::Unsubscribe {
        id: message_id,
        subscription_id,
    });
    if let Err(error) = requester.send(request).await {
        error!("{error}")
    }
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let result = match response {
            Response::Unsubscribe(result) => result,
            response => return error!("unexpected response: {response:?}"),
        };
        if let Err(error) = result {
            error!("Failed to unsubscribe: {}", error)
        };
    });
}
