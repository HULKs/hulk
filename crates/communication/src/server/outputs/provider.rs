use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    num::Wrapping,
    sync::Arc,
};

use bincode::{DefaultOptions, Options};
use framework::{Reader, Writer};
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    select, spawn,
    sync::{
        mpsc::{channel, Sender},
        Notify,
    },
    task::JoinHandle,
};

use crate::{
    messages::{
        BinaryOutputsResponse, BinaryResponse, Format, OutputsRequest, Response,
        TextualDataOrBinaryReference, TextualOutputsResponse, TextualResponse,
    },
    server::{client::Client, client_request::ClientRequest},
};

use super::{Request, Subscription};

pub fn provider<Outputs>(
    outputs_sender: Sender<Request>,
    cycler_instance: &'static str,
    outputs_changed: Arc<Notify>,
    outputs_reader: Reader<Outputs>,
    subscribed_outputs_writer: Writer<HashSet<String>>,
) -> JoinHandle<()>
where
    Outputs: SerializeHierarchy + Send + Sync + 'static,
{
    spawn(async move {
        let (request_sender, mut request_receiver) = channel(1);

        outputs_sender
            .send(Request::RegisterCycler {
                cycler_instance: cycler_instance.to_string(),
                fields: Outputs::get_fields(),
                request_sender,
            })
            .await
            .expect("receiver should always wait for all senders");
        drop(outputs_sender);

        let mut subscriptions = HashMap::new();
        let mut next_binary_reference_id = Wrapping(0);
        loop {
            let subscriptions_state = select! {
                request = request_receiver.recv() => {
                    match request {
                        Some(request) => {
                            handle_client_request::<Outputs>(
                                request,
                                cycler_instance,
                                &mut subscriptions,
                            ).await
                        },
                        None => break,
                    }
                },
                _ = outputs_changed.notified() => {
                    handle_notified_output(&outputs_reader, &mut subscriptions, &mut next_binary_reference_id).await
                },
            };
            if subscriptions_state == SubscriptionsState::Changed {
                write_subscribed_outputs_from_subscriptions(
                    &mut subscriptions,
                    &subscribed_outputs_writer,
                );
            }
        }
    })
}

#[derive(Clone, Copy, PartialEq)]
enum SubscriptionsState {
    Changed,
    Unchanged,
}

async fn handle_client_request<Outputs>(
    request: ClientRequest<OutputsRequest>,
    cycler_instance: &'static str,
    subscriptions: &mut HashMap<(Client, usize), Subscription>,
) -> SubscriptionsState
where
    Outputs: SerializeHierarchy,
{
    let is_get_next = matches!(request.request, OutputsRequest::GetNext { .. });
    match request.request {
        OutputsRequest::GetFields { .. } => {
            panic!("GetFields should be answered by output router");
        }
        OutputsRequest::GetNext {
            id,
            cycler_instance: received_cycler_instance,
            path,
            format,
        }
        | OutputsRequest::Subscribe {
            id,
            cycler_instance: received_cycler_instance,
            path,
            format,
        } => {
            assert_eq!(cycler_instance, received_cycler_instance);
            if Outputs::exists(&path) {
                match subscriptions.entry((request.client.clone(), id)) {
                    Entry::Occupied(_) => {
                        let error_message = format!("already subscribed with id {id}");
                        request
                            .client
                            .response_sender
                            .send(Response::Textual(TextualResponse::Outputs(
                                if is_get_next {
                                    TextualOutputsResponse::GetNext {
                                        id,
                                        result: Err(error_message),
                                    }
                                } else {
                                    TextualOutputsResponse::Subscribe {
                                        id,
                                        result: Err(error_message),
                                    }
                                },
                            )))
                            .await
                            .expect("receiver should always wait for all senders");
                        SubscriptionsState::Unchanged
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(Subscription {
                            path,
                            format,
                            once: is_get_next,
                        });
                        if !is_get_next {
                            request
                                .client
                                .response_sender
                                .send(Response::Textual(TextualResponse::Outputs(
                                    TextualOutputsResponse::Subscribe { id, result: Ok(()) },
                                )))
                                .await
                                .expect("receiver should always wait for all senders");
                        }
                        SubscriptionsState::Changed
                    }
                }
            } else {
                request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Outputs(
                        TextualOutputsResponse::Subscribe {
                            id,
                            result: Err(format!("path {path:?} does not exist")),
                        },
                    )))
                    .await
                    .expect("receiver should always wait for all senders");
                SubscriptionsState::Unchanged
            }
        }
        OutputsRequest::Unsubscribe {
            id,
            subscription_id,
        } => {
            if subscriptions
                .remove(&(request.client.clone(), subscription_id))
                .is_none()
            {
                request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Outputs(
                        TextualOutputsResponse::Unsubscribe {
                            id,
                            result: Err(format!(
                                "never subscribed with subscription id {subscription_id}"
                            )),
                        },
                    )))
                    .await
                    .expect("receiver should always wait for all senders");
                SubscriptionsState::Unchanged
            } else {
                request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Outputs(
                        TextualOutputsResponse::Unsubscribe { id, result: Ok(()) },
                    )))
                    .await
                    .expect("receiver should always wait for all senders");
                SubscriptionsState::Changed
            }
        }
        OutputsRequest::UnsubscribeEverything => {
            let amount_of_subscriptions_before = subscriptions.len();
            subscriptions
                .retain(|(client, _subscription_id), _subscription| &request.client != client);
            if subscriptions.len() != amount_of_subscriptions_before {
                SubscriptionsState::Changed
            } else {
                SubscriptionsState::Unchanged
            }
        }
    }
}

fn write_subscribed_outputs_from_subscriptions(
    subscriptions: &mut HashMap<(Client, usize), Subscription>,
    subscribed_outputs_writer: &Writer<HashSet<String>>,
) {
    let subscribed_outputs = subscriptions
        .values()
        .map(|subscription| subscription.path.clone())
        .collect();
    let mut subscribed_outputs_slot = subscribed_outputs_writer.next();
    *subscribed_outputs_slot = subscribed_outputs;
}

async fn handle_notified_output(
    outputs_reader: &Reader<impl SerializeHierarchy>,
    subscriptions: &mut HashMap<(Client, usize), Subscription>,
    next_binary_reference_id: &mut Wrapping<usize>,
) -> SubscriptionsState {
    let mut textual_get_next_items = HashMap::new();
    let mut textual_subscribed_items: HashMap<
        Client,
        HashMap<usize, TextualDataOrBinaryReference>,
    > = HashMap::new();
    let mut binary_get_next_items = HashMap::new();
    let mut binary_subscribed_items: HashMap<Client, HashMap<usize, Vec<u8>>> = HashMap::new();
    let mut subscriptions_state = SubscriptionsState::Unchanged;
    {
        let output = outputs_reader.next();
        subscriptions.retain(|(client, subscription_id), subscription| {
            let data = match subscription.format {
                Format::Textual => {
                    let data = match output
                        .serialize_path(&subscription.path, serde_json::value::Serializer)
                    {
                        Ok(data) => data,
                        Err(error) => {
                            error!("failed to serialize {:?}: {error:?}", subscription.path);
                            return true;
                        }
                    };
                    TextualDataOrBinaryReference::TextualData { data }
                }
                Format::Binary => {
                    let mut data = Vec::new();
                    let options = DefaultOptions::new()
                        .with_fixint_encoding()
                        .allow_trailing_bytes();
                    let mut serializer = bincode::Serializer::new(&mut data, options);
                    if let Err(error) = output.serialize_path(&subscription.path, &mut serializer) {
                        error!("failed to serialize {:?}: {error:?}", subscription.path);
                        return true;
                    }
                    let reference_id = next_binary_reference_id.0;
                    *next_binary_reference_id += 1;
                    if subscription.once {
                        binary_get_next_items.insert(
                            client.clone(),
                            BinaryOutputsResponse::GetNext { reference_id, data },
                        );
                    } else {
                        binary_subscribed_items
                            .entry(client.clone())
                            .or_default()
                            .insert(reference_id, data);
                    }
                    TextualDataOrBinaryReference::BinaryReference { reference_id }
                }
            };
            if subscription.once {
                textual_get_next_items.insert((client.clone(), *subscription_id), data);
                subscriptions_state = SubscriptionsState::Changed;
                false
            } else {
                textual_subscribed_items
                    .entry(client.clone())
                    .or_default()
                    .insert(*subscription_id, data);
                true
            }
        });
    }
    let send_results: Vec<_> = FuturesUnordered::from_iter(
        textual_get_next_items
            .into_iter()
            .map(|((client, subscription_id), data)| {
                (
                    client.response_sender,
                    Response::Textual(TextualResponse::Outputs(TextualOutputsResponse::GetNext {
                        id: subscription_id,
                        result: Ok(data),
                    })),
                )
            })
            .chain(textual_subscribed_items.into_iter().map(|(client, items)| {
                (
                    client.response_sender,
                    Response::Textual(TextualResponse::Outputs(
                        TextualOutputsResponse::SubscribedData { items },
                    )),
                )
            }))
            .chain(binary_get_next_items.into_iter().map(|(client, response)| {
                (
                    client.response_sender,
                    Response::Binary(BinaryResponse::Outputs(response)),
                )
            }))
            .chain(
                binary_subscribed_items
                    .into_iter()
                    .map(|(client, referenced_items)| {
                        (
                            client.response_sender,
                            Response::Binary(BinaryResponse::Outputs(
                                BinaryOutputsResponse::SubscribedData { referenced_items },
                            )),
                        )
                    }),
            )
            .map(|(response_sender, data)| async move { response_sender.send(data).await }),
    )
    .collect()
    .await;
    for result in send_results.into_iter() {
        if let Err(error) = result {
            error!("failed to send data to client: {error:?}");
        }
    }
    subscriptions_state
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, time::Duration};

    use bincode::serialize;
    use framework::multiple_buffer_with_slots;
    use serde::{de::Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;
    use serialize_hierarchy::Error;
    use tokio::{sync::mpsc::error::TryRecvError, task::yield_now, time::timeout};

    use crate::messages::Format;

    use super::*;

    struct OutputsFake<T> {
        existing_fields: HashMap<String, T>,
    }

    impl<T> SerializeHierarchy for OutputsFake<T>
    where
        for<'a> T: Deserialize<'a> + Serialize,
    {
        fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
        where
            S: Serializer,
        {
            self.existing_fields
                .get(path)
                .ok_or(Error::UnexpectedPathSegment {
                    segment: path.to_string(),
                })?
                .serialize(serializer)
                .map_err(Error::SerializationFailed)
        }

        fn deserialize_path<'de, D>(
            &mut self,
            path: &str,
            deserializer: D,
        ) -> Result<(), Error<D::Error>>
        where
            D: Deserializer<'de>,
        {
            self.existing_fields.insert(
                path.to_string(),
                T::deserialize(deserializer).map_err(Error::DeserializationFailed)?,
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

    async fn get_registered_request_sender_from_provider(
        cycler_instance: &'static str,
        outputs_changed: Arc<Notify>,
        output: Reader<impl SerializeHierarchy + Send + Sync + 'static>,
    ) -> (
        JoinHandle<()>,
        BTreeSet<String>,
        Sender<ClientRequest<OutputsRequest>>,
        Reader<HashSet<String>>,
    ) {
        let (outputs_sender, mut outputs_receiver) = channel(1);
        let (subscribed_outputs_writer, subscribed_outputs_reader) = multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let join_handle = provider(
            outputs_sender,
            cycler_instance,
            outputs_changed,
            output,
            subscribed_outputs_writer,
        );
        let (fields, request_sender) = timeout(Duration::from_secs(1), async move {
            let Some(request) = outputs_receiver.recv().await else {
                panic!("expected request");
            };
            let Request::RegisterCycler {
                cycler_instance: cycler_instance_to_register,
                fields,
                request_sender,
            } = request
            else {
                panic!("expected Request::RegisterCycler");
            };
            assert_eq!(cycler_instance, cycler_instance_to_register);
            assert!(outputs_receiver.recv().await.is_none());
            (fields, request_sender)
        })
        .await
        .unwrap();
        (
            join_handle,
            fields,
            request_sender,
            subscribed_outputs_reader,
        )
    }

    #[tokio::test]
    async fn provider_registers_itself_at_router() {
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                "CyclerInstance",
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn fields_are_collected() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake::<()> {
            existing_fields: Default::default(),
        }]);

        let (provider_task, fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        assert_eq!(
            fields,
            ["a".to_string(), "a.b".to_string(), "a.b.c".to_string()].into(),
        );

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_same_client_ids() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const ID: usize = 42;
        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: ID,
                    cycler_instance: cycler_instance.clone(),
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: ID,
                    cycler_instance,
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_different_client_ids() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const ID: usize = 42;
        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: ID,
                    cycler_instance: cycler_instance.clone(),
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: ID,
                    cycler_instance,
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_different_subscription_ids_and_same_client_ids() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: 42,
                    cycler_instance: cycler_instance.clone(),
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: 42,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: 1337,
                    cycler_instance,
                    path: path.clone(),
                    format,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_unknown_subscription_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_twice_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let path = "a.b.c".to_string();
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: SUBSCRIPTION_ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_after_unsubscribe_everything_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [("a.b.c".to_string(), 42)].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed,
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: 42,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: 42,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::UnsubscribeEverything,
                client: Client {
                    id: 1337,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continuing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn textual_data_from_notified_output_is_sent_to_subscribed_client() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed.clone(),
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: SUBSCRIPTION_ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        outputs_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Outputs(
                TextualOutputsResponse::SubscribedData {
                    items: [(
                        SUBSCRIPTION_ID,
                        TextualDataOrBinaryReference::TextualData { data: value }
                    )]
                    .into()
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        outputs_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn binary_data_from_notified_output_is_sent_to_subscribed_client() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = vec![42, 1, 3, 3, 7];
        let serialized_value = serialize(&value).unwrap();
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed.clone(),
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Binary,
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: SUBSCRIPTION_ID,
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
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        outputs_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        let Response::Textual(TextualResponse::Outputs(TextualOutputsResponse::SubscribedData {
            items,
        })) = subscribed_data
        else {
            panic!("unexpected subscribed data: {subscribed_data:?}");
        };
        assert_eq!(items.len(), 1);
        let Some(TextualDataOrBinaryReference::BinaryReference { reference_id }) =
            items.get(&SUBSCRIPTION_ID)
        else {
            panic!("an item with subscription ID {SUBSCRIPTION_ID} should exist");
        };
        let binary_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            binary_data,
            Response::Binary(BinaryResponse::Outputs(
                BinaryOutputsResponse::SubscribedData {
                    referenced_items: [(*reference_id, serialized_value)].into()
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
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
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        outputs_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_subscriptions_duplicate_data() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed.clone(),
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender0, mut response_receiver0) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender0.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver0.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: SUBSCRIPTION_ID,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver0.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        let (response_sender1, mut response_receiver1) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender1.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver1.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Subscribe {
                        id: SUBSCRIPTION_ID,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver1.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        outputs_changed.notify_one();
        let subscribed_data = response_receiver0.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Outputs(
                TextualOutputsResponse::SubscribedData {
                    items: [(
                        SUBSCRIPTION_ID,
                        TextualDataOrBinaryReference::TextualData {
                            data: value.clone()
                        }
                    )]
                    .into()
                }
            )),
        );
        match response_receiver0.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        let subscribed_data = response_receiver1.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Outputs(
                TextualOutputsResponse::SubscribedData {
                    items: [(
                        SUBSCRIPTION_ID,
                        TextualDataOrBinaryReference::TextualData { data: value }
                    )]
                    .into()
                }
            )),
        );
        match response_receiver1.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path.clone()]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender0.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver0.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
                        id: 1337,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver0.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        request_sender
            .send(ClientRequest {
                request: OutputsRequest::Unsubscribe {
                    id: 1337,
                    subscription_id: SUBSCRIPTION_ID,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender1.clone(),
                },
            })
            .await
            .unwrap();
        let response = response_receiver1.recv().await.unwrap();
        assert!(
            matches!(
                response,
                Response::Textual(TextualResponse::Outputs(
                    TextualOutputsResponse::Unsubscribe {
                        id: 1337,
                        result: Ok(()),
                    }
                ))
            ),
            "unexpected {response:?}",
        );
        match response_receiver1.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        outputs_changed.notify_one();
        match response_receiver0.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        match response_receiver1.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn textual_get_next_forwards_data_once() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed.clone(),
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::GetNext {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Textual,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continuing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        outputs_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Outputs(TextualOutputsResponse::GetNext {
                id: SUBSCRIPTION_ID,
                result: Ok(TextualDataOrBinaryReference::TextualData { data: value })
            })),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        outputs_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn binary_get_next_forwards_data_once() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = vec![42, 1, 3, 3, 7];
        let serialized_value = serialize(&value).unwrap();
        let outputs_changed = Arc::new(Notify::new());
        let (_output_writer, outputs_reader) = multiple_buffer_with_slots([OutputsFake {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender, subscribed_outputs_reader) =
            get_registered_request_sender_from_provider(
                cycler_instance,
                outputs_changed.clone(),
                outputs_reader,
            )
            .await;
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: OutputsRequest::GetNext {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: path.clone(),
                    format: Format::Binary,
                },
                client: Client {
                    id: client_id,
                    response_sender: response_sender.clone(),
                },
            })
            .await
            .unwrap();

        // ensure that we are subscribed before continuing because GetNext has no synchronous response
        yield_now().await;

        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(
            *subscribed_outputs_reader.next(),
            HashSet::from_iter([path]),
        );

        outputs_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        let Response::Textual(TextualResponse::Outputs(TextualOutputsResponse::GetNext {
            id: SUBSCRIPTION_ID,
            result: Ok(TextualDataOrBinaryReference::BinaryReference { reference_id }),
        })) = subscribed_data
        else {
            panic!("unexpected subscribed data: {subscribed_data:?}");
        };
        let binary_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            binary_data,
            Response::Binary(BinaryResponse::Outputs(BinaryOutputsResponse::GetNext {
                reference_id,
                data: serialized_value,
            })),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        outputs_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        assert_eq!(*subscribed_outputs_reader.next(), HashSet::new());

        drop(request_sender);
        provider_task.await.unwrap();
    }
}
