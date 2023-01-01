use std::collections::{hash_map::Entry, BTreeMap, HashMap};

use tokio::{
    spawn,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::server::messages::{
    DatabaseRequest, Path, Response, TextualDatabaseResponse, TextualResponse, Type,
};

use super::{Client, ClientRequest, Request};

pub fn router(
    keep_running: CancellationToken,
    mut request_receiver: Receiver<Request>,
) -> JoinHandle<()> {
    spawn(async move {
        let mut request_channels_of_cyclers = HashMap::new();
        let mut cached_cycler_instances = HashMap::new();

        println!("Entering databases loop...");
        while let Some(request) = request_receiver.recv().await {
            println!("databases: request: {request:?}");
            match request {
                Request::ClientRequest(request) => {
                    println!("databases: request: {request:?}");
                    forward_client_request_to_provider(
                        request,
                        &request_channels_of_cyclers,
                        &mut cached_cycler_instances,
                    )
                    .await
                }
                Request::RegisterCycler {
                    cycler_instance,
                    fields,
                    request_sender,
                } => {
                    request_channels_of_cyclers.insert(cycler_instance, (fields, request_sender));
                }
            }
        }
        println!("Exited databases loop.");
    })
}

async fn forward_client_request_to_provider(
    request: ClientRequest,
    request_channels_of_cyclers: &HashMap<String, (BTreeMap<Path, Type>, Sender<ClientRequest>)>,
    cached_cycler_instances: &mut HashMap<(Client, usize), String>,
) {
    match &request.request {
        DatabaseRequest::GetHierarchy { id } => {
            // TODO: directly generate BTreeMap in SerializeHierarchy
            let _ = request
                .client
                .response_sender
                .send(Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::GetHierarchy {
                        id: *id,
                        fields: request_channels_of_cyclers
                            .iter()
                            .map(|(cycler_instance, (fields, _request_sender))| {
                                (cycler_instance.clone(), fields.clone())
                            })
                            .collect(),
                    },
                )))
                .await;
        }
        DatabaseRequest::GetNext {
            id,
            cycler_instance,
            ..
        }
        | DatabaseRequest::Subscribe {
            id,
            cycler_instance,
            ..
        } => {
            if matches!(request.request, DatabaseRequest::Subscribe { .. }) {
                cached_cycler_instances
                    .insert((request.client.clone(), *id), cycler_instance.clone());
            }

            match request_channels_of_cyclers.get(cycler_instance) {
                Some((_fields, request_channel)) => {
                    let _ = request_channel.send(request).await;
                }
                None => {
                    let _ = request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Databases(
                            TextualDatabaseResponse::Subscribe {
                                id: *id,
                                result: Err(format!("unknown cycler_instance {cycler_instance:?}")),
                            },
                        )))
                        .await;
                    return;
                }
            }
        }
        DatabaseRequest::Unsubscribe {
            id,
            subscription_id,
        } => {
            let cycler_instance = match cached_cycler_instances
                .entry((request.client.clone(), *subscription_id))
            {
                Entry::Occupied(entry) => entry.remove(),
                Entry::Vacant(_) => {
                    let _ = request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Databases(
                            TextualDatabaseResponse::Subscribe {
                                id: *id,
                                result: Err(format!("unknown subscription ID {subscription_id}")),
                            },
                        )))
                        .await;
                    return;
                }
            };

            match request_channels_of_cyclers.get(&cycler_instance) {
                Some((_fields, request_channel)) => {
                    let _ = request_channel.send(request).await;
                }
                None => {
                    let _ = request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Databases(
                            TextualDatabaseResponse::Subscribe {
                                id: *id,
                                result: Err(format!("unknown cycler_instance {cycler_instance:?}")),
                            },
                        )))
                        .await;
                    return;
                }
            }
        }
    }
}
