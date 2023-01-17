use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap},
    sync::Arc,
};

use framework::Reader;
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use serialize_hierarchy::{SerializeHierarchy, TextualSerializer};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{Receiver, Sender},
        Notify,
    },
    task::JoinHandle,
};

use crate::{
    messages::{ParametersRequest, ParametersResponse, Path, Response, TextualResponse},
    server::{client::Client, client_request::ClientRequest},
};

use super::StorageRequest;

pub fn subscriptions<Parameters>(
    mut request_receiver: Receiver<ClientRequest<ParametersRequest>>,
    parameters_reader: Reader<Parameters>,
    parameters_changed: Arc<Notify>,
    storage_request_sender: Sender<StorageRequest>,
) -> JoinHandle<()>
where
    Parameters: Send + SerializeHierarchy + Sync + 'static,
{
    spawn(async move {
        let fields = Parameters::get_fields();

        let mut subscriptions = HashMap::new();
        loop {
            select! {
                request = request_receiver.recv() => {
                    let Some(request) = request else {
                        break;
                    };
                    handle_request(
                        request,
                        &parameters_reader,
                        &storage_request_sender,
                        &mut subscriptions,
                        &fields,
                    ).await;
                },
                _ = parameters_changed.notified() => {
                    handle_changed_parameters(&parameters_reader, &subscriptions).await;
                }
            }
        }
    })
}

async fn handle_request<Parameters>(
    request: ClientRequest<ParametersRequest>,
    parameters_reader: &Reader<Parameters>,
    storage_request_sender: &Sender<StorageRequest>,
    subscriptions: &mut HashMap<(Client, usize), Path>,
    fields: &BTreeSet<String>,
) where
    Parameters: SerializeHierarchy,
{
    match request.request {
        ParametersRequest::GetFields { id } => {
            respond(
                request,
                ParametersResponse::GetFields {
                    id,
                    fields: fields.clone(),
                },
            )
            .await;
        }
        ParametersRequest::GetCurrent { id, ref path } => {
            let data = {
                let parameters = parameters_reader.next();
                parameters.serialize_path::<TextualSerializer>(path)
            };
            let data = match data {
                Ok(data) => data,
                Err(error) => {
                    respond(
                        request,
                        ParametersResponse::GetCurrent {
                            id,
                            result: Err(format!("failed to serialize: {error:?}")),
                        },
                    )
                    .await;
                    return;
                }
            };
            respond(
                request,
                ParametersResponse::GetCurrent {
                    id,
                    result: Ok(data),
                },
            )
            .await;
        }
        ParametersRequest::Subscribe { id, ref path } => {
            if !Parameters::exists(path) {
                let error_message = format!("path {path:?} does not exist");
                respond(
                    request,
                    ParametersResponse::Subscribe {
                        id,
                        result: Err(error_message),
                    },
                )
                .await;
                return;
            }

            let response = match subscriptions.entry((request.client.clone(), id)) {
                Entry::Occupied(_) => ParametersResponse::Subscribe {
                    id,
                    result: Err(format!("already subscribed with id {id}")),
                },
                Entry::Vacant(entry) => {
                    entry.insert(path.to_string());
                    ParametersResponse::Subscribe { id, result: Ok(()) }
                }
            };

            let data = {
                let parameters = parameters_reader.next();
                parameters.serialize_path::<TextualSerializer>(path)
            };
            let data = match data {
                Ok(data) => data,
                Err(error) => {
                    respond(
                        request,
                        ParametersResponse::Subscribe {
                            id,
                            result: Err(format!("failed to serialize: {error:?}")),
                        },
                    )
                    .await;
                    return;
                }
            };
            respond(request.clone(), response).await;
            request
                .client
                .response_sender
                .send(Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::SubscribedData {
                        subscription_id: id,
                        data,
                    },
                )))
                .await
                .expect("receiver should always wait for all senders");
        }
        ParametersRequest::Unsubscribe {
            id,
            subscription_id,
        } => {
            if subscriptions
                .remove(&(request.client.clone(), subscription_id))
                .is_none()
            {
                respond(
                    request,
                    ParametersResponse::Unsubscribe {
                        id,
                        result: Err(format!(
                            "never subscribed with subscription id {subscription_id}"
                        )),
                    },
                )
                .await;
            } else {
                respond(
                    request,
                    ParametersResponse::Unsubscribe { id, result: Ok(()) },
                )
                .await;
            }
        }
        ParametersRequest::UnsubscribeEverything => {
            subscriptions
                .retain(|(client, _subscription_id), _subscription| &request.client != client);
        }
        ParametersRequest::Update { id, path, data } => {
            storage_request_sender
                .send(StorageRequest::UpdateParameter {
                    client: request.client,
                    id,
                    path,
                    data,
                })
                .await
                .expect("receiver should always wait for all senders");
        }
        ParametersRequest::LoadFromDisk { id } => {
            storage_request_sender
                .send(StorageRequest::LoadFromDisk {
                    client: request.client,
                    id,
                })
                .await
                .expect("receiver should always wait for all senders");
        }
        ParametersRequest::StoreToDisk { id } => {
            storage_request_sender
                .send(StorageRequest::StoreToDisk {
                    client: request.client,
                    id,
                })
                .await
                .expect("receiver should always wait for all senders");
        }
    }
}

async fn respond(request: ClientRequest<ParametersRequest>, response: ParametersResponse) {
    request
        .client
        .response_sender
        .send(Response::Textual(TextualResponse::Parameters(response)))
        .await
        .expect("receiver should always wait for all senders");
}

async fn handle_changed_parameters<Parameters>(
    parameters_reader: &Reader<Parameters>,
    subscriptions: &HashMap<(Client, usize), Path>,
) where
    Parameters: SerializeHierarchy,
{
    let items: HashMap<_, _> = {
        let parameters = parameters_reader.next();
        subscriptions
            .iter()
            .filter_map(|((client, subscription_id), path)| {
                let data = match parameters.serialize_path::<TextualSerializer>(path) {
                    Ok(data) => data,
                    Err(error) => {
                        error!("failed to serialize {:?}: {error:?}", path);
                        return None;
                    }
                };
                Some(((client.clone(), *subscription_id), data))
            })
            .collect()
    };
    let send_results: Vec<_> = FuturesUnordered::from_iter(
        items
            .into_iter()
            .map(|((client, subscription_id), data)| {
                (
                    client.response_sender,
                    Response::Textual(TextualResponse::Parameters(
                        ParametersResponse::SubscribedData {
                            subscription_id,
                            data,
                        },
                    )),
                )
            })
            .map(|(response_sender, data)| async move { response_sender.send(data).await }),
    )
    .collect()
    .await;
    for result in send_results.into_iter() {
        if let Err(error) = result {
            error!("failed to send data to client: {error:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use framework::multiple_buffer_with_slots;
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json::Value;
    use serialize_hierarchy::{Error, Serializer};
    use tokio::{
        sync::mpsc::{channel, error::TryRecvError},
        task::yield_now,
    };

    use super::*;

    #[tokio::test]
    async fn terminates_on_request_sender_drop() {
        let (request_sender, request_receiver) = channel(1);
        let (_parameters_writer, parameters_reader) = multiple_buffer_with_slots([42usize]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn fields_are_returned() {
        let (request_sender, request_receiver) = channel(1);
        let (_parameters_writer, parameters_reader) = multiple_buffer_with_slots([42usize]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::GetFields { id: 42 },
                client: Client {
                    id: 1337,
                    response_sender,
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert_eq!(
            response,
            Response::Textual(TextualResponse::Parameters(ParametersResponse::GetFields {
                id: 42,
                fields: Default::default(),
            })),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Disconnected) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    struct ParametersFake<T> {
        existing_fields: HashMap<String, T>,
    }

    impl<T> SerializeHierarchy for ParametersFake<T>
    where
        T: DeserializeOwned + Serialize,
    {
        fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
        where
            S: Serializer,
            S::Error: std::error::Error,
        {
            S::serialize(
                self.existing_fields
                    .get(path)
                    .ok_or(Error::UnexpectedPathSegment {
                        segment: path.to_string(),
                    })?,
            )
            .map_err(Error::SerializationFailed)
        }

        fn deserialize_path<S>(
            &mut self,
            path: &str,
            data: S::Serialized,
        ) -> Result<(), Error<S::Error>>
        where
            S: Serializer,
            S::Error: std::error::Error,
        {
            self.existing_fields.insert(
                path.to_string(),
                S::deserialize(data).map_err(Error::DeserializationFailed)?,
            );
            Ok(())
        }

        fn exists(field_path: &str) -> bool {
            field_path == "a.b.c"
        }

        fn get_fields() -> BTreeSet<String> {
            ["a".to_string(), "a.b".to_string(), "a.b.c".to_string()].into()
        }
    }

    #[tokio::test]
    async fn get_current_returns_data() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), value.clone())].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::GetCurrent { id: 42, path },
                client: Client {
                    id: 1337,
                    response_sender,
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert_eq!(
            response,
            Response::Textual(TextualResponse::Parameters(
                ParametersResponse::GetCurrent {
                    id: 42,
                    result: Ok(value),
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Disconnected) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_same_client_ids() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        const ID: usize = 42;
        let path = "a.b.c".to_string();
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: ID,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: ID,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: ID,
                    result: Err(_),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_different_client_ids() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        const ID: usize = 42;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: ID,
                    path: path.clone(),
                },
                client: Client {
                    id: 1337,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: ID,
                    path: path.clone(),
                },
                client: Client {
                    id: 7331,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_different_subscription_ids_and_same_client_ids() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: 42,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: 42,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: 1337,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: 1337,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_unknown_subscription_results_in_error() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Unsubscribe {
                    id: 42,
                    subscription_id: 1337,
                },
                client: Client {
                    id: 1337,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::Unsubscribe {
                        id: 42,
                        result: Err(_),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_twice_results_in_error() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: SUBSCRIPTION_ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::Unsubscribe {
                        id: 1337,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::Unsubscribe {
                        id: 1337,
                        result: Err(_),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_after_unsubscribe_everything_results_in_error() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), 42)].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: SUBSCRIPTION_ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::UnsubscribeEverything,
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continueing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::Unsubscribe {
                        id: 1337,
                        result: Err(_),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn parameter_update_is_forwarded_to_storage() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), value.clone())].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, mut storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let path = "a.b.c".to_string();
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Update {
                    id: 42,
                    path: path.clone(),
                    data: value.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continueing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        let storage_request = storage_request_receiver.recv().await.unwrap();
        assert_eq!(
            storage_request,
            StorageRequest::UpdateParameter {
                client: Client {
                    id: client_id,
                    response_sender,
                },
                id: 42,
                path,
                data: value,
            }
        );

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn load_from_disk_is_forwarded_to_storage() {
        let (request_sender, request_receiver) = channel(1);
        let (_parameters_writer, parameters_reader) = multiple_buffer_with_slots([42]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, mut storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::LoadFromDisk { id: 42 },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continueing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        let storage_request = storage_request_receiver.recv().await.unwrap();
        assert_eq!(
            storage_request,
            StorageRequest::LoadFromDisk {
                client: Client {
                    id: client_id,
                    response_sender,
                },
                id: 42,
            }
        );

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn store_to_disk_is_forwarded_to_storage() {
        let (request_sender, request_receiver) = channel(1);
        let (_parameters_writer, parameters_reader) = multiple_buffer_with_slots([42]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, mut storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed,
            storage_request_sender,
        );

        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::StoreToDisk { id: 42 },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continueing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        let storage_request = storage_request_receiver.recv().await.unwrap();
        assert_eq!(
            storage_request,
            StorageRequest::StoreToDisk {
                client: Client {
                    id: client_id,
                    response_sender,
                },
                id: 42,
            }
        );

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[tokio::test]
    async fn data_from_notified_parameters_is_sent_to_subscribed_client() {
        let (request_sender, request_receiver) = channel(1);
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let (_parameters_writer, parameters_reader) =
            multiple_buffer_with_slots([ParametersFake {
                existing_fields: [(path.clone(), value.clone())].into(),
            }]);
        let parameters_changed = Arc::new(Notify::new());
        let (storage_request_sender, _storage_request_receiver) = channel(1);
        let subscriptions_task = subscriptions(
            request_receiver,
            parameters_reader,
            parameters_changed.clone(),
            storage_request_sender,
        );

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    path: path.clone(),
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(ParametersResponse::Subscribe {
                    id: SUBSCRIPTION_ID,
                    result: Ok(()),
                }))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        parameters_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Parameters(
                ParametersResponse::SubscribedData {
                    subscription_id: SUBSCRIPTION_ID,
                    data: value,
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        request_sender
            .send(ClientRequest {
                request: ParametersRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Parameters(
                    ParametersResponse::Unsubscribe {
                        id: 1337,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }
}
