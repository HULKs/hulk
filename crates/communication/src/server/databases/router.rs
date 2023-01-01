use std::collections::{hash_map::Entry, BTreeMap, HashMap};

use tokio::{
    spawn,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::server::messages::{
    DatabaseRequest, Path, Response, TextualDatabaseResponse, TextualResponse, Type,
};

use super::{Client, ClientRequest, Request};

pub fn router(mut request_receiver: Receiver<Request>) -> JoinHandle<()> {
    spawn(async move {
        let mut request_channels_of_cyclers = HashMap::new();
        let mut cached_cycler_instances = HashMap::new();

        while let Some(request) = request_receiver.recv().await {
            match request {
                Request::ClientRequest(request) => {
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
    })
}

async fn forward_client_request_to_provider(
    request: ClientRequest,
    request_channels_of_cyclers: &HashMap<String, (BTreeMap<Path, Type>, Sender<ClientRequest>)>,
    cached_cycler_instances: &mut HashMap<(Client, usize), String>,
) {
    match &request.request {
        DatabaseRequest::GetFields { id } => {
            // TODO: directly generate BTreeMap in SerializeHierarchy
            let _ = request
                .client
                .response_sender
                .send(Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::GetFields {
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
                    let error_message = format!("unknown cycler_instance {cycler_instance:?}");
                    let _ = request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Databases(
                            if matches!(request.request, DatabaseRequest::GetNext { .. }) {
                                TextualDatabaseResponse::GetNext {
                                    id: *id,
                                    result: Err(error_message),
                                }
                            } else {
                                TextualDatabaseResponse::Subscribe {
                                    id: *id,
                                    result: Err(error_message),
                                }
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
                // TODO: we remove the cache entry despite possible errors during unsubscription
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

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::{channel, error::TryRecvError};

    use crate::server::messages::Format;

    use super::*;

    #[tokio::test]
    async fn terminates_on_request_sender_drop() {
        let (request_sender, request_receiver) = channel(1);
        let router_task = router(request_receiver);

        drop(request_sender);
        router_task.await.unwrap();
    }

    #[tokio::test]
    async fn fields_are_returned() {
        let (request_sender, request_receiver) = channel(1);
        let router_task = router(request_receiver);

        let cycler_instance = "CyclerInstance";
        let fields: BTreeMap<String, String> = [("a.b.c".to_string(), "bool".to_string())].into();
        let (provider_request_sender, _provider_request_receiver) = channel(1);
        request_sender
            .send(Request::RegisterCycler {
                cycler_instance: cycler_instance.to_string(),
                fields: fields.clone(),
                request_sender: provider_request_sender,
            })
            .await
            .unwrap();

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(Request::ClientRequest(ClientRequest {
                request: DatabaseRequest::GetFields { id: 42 },
                client: Client {
                    id: 1337,
                    response_sender,
                },
            }))
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert_eq!(
            response,
            Response::Textual(TextualResponse::Databases(
                TextualDatabaseResponse::GetFields {
                    id: 42,
                    fields: [(cycler_instance.to_string(), fields)].into()
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Disconnected) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        router_task.await.unwrap();
    }

    #[tokio::test]
    async fn unknown_cycler_instance_results_in_error() {
        let (request_sender, request_receiver) = channel(1);
        let router_task = router(request_receiver);

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(Request::ClientRequest(ClientRequest {
                request: DatabaseRequest::GetNext {
                    id: 42,
                    cycler_instance: "CyclerInstance".to_string(),
                    path: "a.b.c".to_string(),
                    format: Format::Textual,
                },
                client: Client {
                    id: 1337,
                    response_sender,
                },
            }))
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::GetNext {
                        id: 42,
                        result: Err(_),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Disconnected) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        router_task.await.unwrap();
    }

    #[tokio::test]
    async fn client_request_is_forwarded() {
        let (request_sender, request_receiver) = channel(1);
        let router_task = router(request_receiver);

        let cycler_instance = "CyclerInstance";
        let (provider_request_sender, mut provider_request_receiver) = channel(1);
        request_sender
            .send(Request::RegisterCycler {
                cycler_instance: cycler_instance.to_string(),
                fields: Default::default(),
                request_sender: provider_request_sender,
            })
            .await
            .unwrap();

        let (response_sender, _response_receiver) = channel(1);
        let sent_client_request = ClientRequest {
            request: DatabaseRequest::GetNext {
                id: 42,
                cycler_instance: "CyclerInstance".to_string(),
                path: "a.b.c".to_string(),
                format: Format::Textual,
            },
            client: Client {
                id: 1337,
                response_sender,
            },
        };
        request_sender
            .send(Request::ClientRequest(sent_client_request.clone()))
            .await
            .unwrap();
        let forwarded_client_request = provider_request_receiver.recv().await.unwrap();
        assert_eq!(forwarded_client_request, sent_client_request);

        drop(request_sender);
        router_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_request_is_forwarded_to_subscribe_request_cycler_instance() {
        let (request_sender, request_receiver) = channel(1);
        let router_task = router(request_receiver);

        let cycler_instance = "CyclerInstance";
        let (provider_request_sender, mut provider_request_receiver) = channel(1);
        request_sender
            .send(Request::RegisterCycler {
                cycler_instance: cycler_instance.to_string(),
                fields: Default::default(),
                request_sender: provider_request_sender,
            })
            .await
            .unwrap();

        let (response_sender, _response_receiver) = channel(1);
        let client = Client {
            id: 1337,
            response_sender,
        };
        let sent_client_request = ClientRequest {
            request: DatabaseRequest::Subscribe {
                id: 42,
                cycler_instance: "CyclerInstance".to_string(),
                path: "a.b.c".to_string(),
                format: Format::Textual,
            },
            client: client.clone(),
        };
        request_sender
            .send(Request::ClientRequest(sent_client_request.clone()))
            .await
            .unwrap();
        let forwarded_client_request = provider_request_receiver.recv().await.unwrap();
        assert_eq!(forwarded_client_request, sent_client_request);

        let sent_client_request = ClientRequest {
            request: DatabaseRequest::Unsubscribe {
                id: 1337,
                subscription_id: 42,
            },
            client,
        };
        request_sender
            .send(Request::ClientRequest(sent_client_request.clone()))
            .await
            .unwrap();
        let forwarded_client_request = provider_request_receiver.recv().await.unwrap();
        assert_eq!(forwarded_client_request, sent_client_request);

        drop(request_sender);
        router_task.await.unwrap();
    }
}
