use std::collections::{hash_map::Entry, BTreeSet, HashMap};

use tokio::{
    spawn,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    messages::{
        OutputsRequest, Path, Response, TextualOutputsResponse, TextualResponse,
    },
    server::{client::Client, client_request::ClientRequest},
};

use super::Request;

pub fn router(mut request_receiver: Receiver<Request>) -> JoinHandle<()> {
    spawn(async move {
        let mut request_channels_of_cyclers = HashMap::new();
        let mut cached_cycler_instances = HashMap::new();

        while let Some(request) = request_receiver.recv().await {
            match request {
                Request::ClientRequest(request) => {
                    handle_request(
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

async fn handle_request(
    request: ClientRequest<OutputsRequest>,
    request_channels_of_cyclers: &HashMap<
        String,
        (BTreeSet<Path>, Sender<ClientRequest<OutputsRequest>>),
    >,
    cached_cycler_instances: &mut HashMap<(Client, usize), String>,
) {
    match &request.request {
        OutputsRequest::GetFields { id } => {
            request
                .client
                .response_sender
                .send(Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::GetFields {
                        id: *id,
                        fields: request_channels_of_cyclers
                            .iter()
                            .map(|(cycler_instance, (fields, _request_sender))| {
                                (cycler_instance.clone(), fields.clone())
                            })
                            .collect(),
                    },
                )))
                .await
                .expect("receiver should always wait for all senders");
        }
        OutputsRequest::GetNext { id, path, .. } | OutputsRequest::Subscribe { id, path, .. } => {
            let cycler_instance = match path.split_once('.') {
                Some((cycler_instance, _)) => cycler_instance,
                None => {
                    let error_message = format!("cannot parse path {path}");
                    request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Outputs(
                            if matches!(request.request, OutputsRequest::GetNext { .. }) {
                                TextualOutputsResponse::GetNext {
                                    id: *id,
                                    result: Err(error_message),
                                }
                            } else {
                                TextualOutputsResponse::Subscribe {
                                    id: *id,
                                    result: Err(error_message),
                                }
                            },
                        )))
                        .await
                        .expect("receiver should always wait for all senders");
                    return;
                }
            };

            if matches!(request.request, OutputsRequest::Subscribe { .. }) {
                cached_cycler_instances
                    .insert((request.client.clone(), *id), cycler_instance.to_owned());
            }

            match request_channels_of_cyclers.get(cycler_instance) {
                Some((_fields, request_channel)) => {
                    request_channel
                        .send(request)
                        .await
                        .expect("receiver should always wait for all senders");
                }
                None => {
                    let error_message = format!("unknown cycler_instance {cycler_instance:?}");
                    request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Outputs(
                            if matches!(request.request, OutputsRequest::GetNext { .. }) {
                                TextualOutputsResponse::GetNext {
                                    id: *id,
                                    result: Err(error_message),
                                }
                            } else {
                                TextualOutputsResponse::Subscribe {
                                    id: *id,
                                    result: Err(error_message),
                                }
                            },
                        )))
                        .await
                        .expect("receiver should always wait for all senders");
                }
            }
        }
        OutputsRequest::Unsubscribe {
            id,
            subscription_id,
        } => {
            let cycler_instance = match cached_cycler_instances
                .entry((request.client.clone(), *subscription_id))
            {
                Entry::Occupied(entry) => entry.remove(),
                Entry::Vacant(_) => {
                    request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Outputs(
                            TextualOutputsResponse::Unsubscribe {
                                id: *id,
                                result: Err(format!("unknown subscription ID {subscription_id}")),
                            },
                        )))
                        .await
                        .expect("receiver should always wait for all senders");
                    return;
                }
            };

            match request_channels_of_cyclers.get(&cycler_instance) {
                Some((_fields, request_channel)) => {
                    request_channel
                        .send(request)
                        .await
                        .expect("receiver should always wait for all senders");
                }
                None => {
                    request
                        .client
                        .response_sender
                        .send(Response::Textual(TextualResponse::Outputs(
                            TextualOutputsResponse::Unsubscribe {
                                id: *id,
                                result: Err(format!("unknown cycler_instance {cycler_instance:?}")),
                            },
                        )))
                        .await
                        .expect("receiver should always wait for all senders");
                }
            }
        }
        OutputsRequest::UnsubscribeEverything => {
            cached_cycler_instances
                .retain(|(client, _subscription_id), _cycler_instance| client != &request.client);
            for (_fields, request_channel) in request_channels_of_cyclers.values() {
                request_channel
                    .send(request.clone())
                    .await
                    .expect("receiver should always wait for all senders");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::{channel, error::TryRecvError};

    use crate::messages::Format;

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
        let fields: BTreeSet<String> = ["a.b.c".to_string()].into();
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
                request: OutputsRequest::GetFields { id: 42 },
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
            Response::Textual(TextualResponse::Outputs(
                TextualOutputsResponse::GetFields {
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
                request: OutputsRequest::GetNext {
                    id: 42,
                    path: "CyclerInstance.a.b.c".to_string(),
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
                Response::Textual(TextualResponse::Outputs(TextualOutputsResponse::GetNext {
                    id: 42,
                    result: Err(_),
                }))
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
            request: OutputsRequest::GetNext {
                id: 42,
                path: "CyclerInstance.a.b.c".to_string(),
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
            request: OutputsRequest::Subscribe {
                id: 42,
                path: "CyclerInstance.a.b.c".to_string(),
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
            request: OutputsRequest::Unsubscribe {
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
