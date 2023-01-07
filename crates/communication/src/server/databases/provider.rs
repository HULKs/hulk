use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    sync::Arc,
};

use framework::Reader;
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use serde_json::Value;
use serialize_hierarchy::{HierarchyType, SerializeHierarchy};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{channel, Sender},
        Notify,
    },
    task::JoinHandle,
};

use crate::server::messages::{
    DatabaseRequest, Response, TextualDataOrBinaryReference, TextualDatabaseResponse,
    TextualResponse,
};

use super::{Client, ClientRequest, Request, Subscription};

pub fn provider<Database>(
    databases_sender: Sender<Request>,
    cycler_instance: &'static str,
    database_changed: Arc<Notify>,
    database_reader: Reader<Database>,
) -> JoinHandle<()>
where
    Database: SerializeHierarchy + Send + Sync + 'static,
{
    spawn(async move {
        let (request_sender, mut request_receiver) = channel(1);

        databases_sender
            .send(Request::RegisterCycler {
                cycler_instance: cycler_instance.to_string(),
                fields: get_paths_from_hierarchy(Default::default(), Database::get_hierarchy()),
                request_sender,
            })
            .await
            .expect("receiver should always wait for all senders");
        drop(databases_sender);

        let mut subscriptions = HashMap::new();
        loop {
            select! {
                request = request_receiver.recv() => match request {
                    Some(request) => handle_client_request::<Database>(
                        request,
                        cycler_instance,
                        &mut subscriptions,
                    ).await,
                    None => break,
                },
                _ = database_changed.notified() => {
                    handle_notified_database(&database_reader, &mut subscriptions).await;
                },
            }
        }
    })
}

async fn handle_client_request<Database>(
    request: ClientRequest,
    cycler_instance: &'static str,
    subscriptions: &mut HashMap<(Client, usize), Subscription>,
) where
    Database: SerializeHierarchy,
{
    let is_get_next = matches!(request.request, DatabaseRequest::GetNext { .. });
    match request.request {
        DatabaseRequest::GetFields { .. } => {
            panic!("GetFields should be answered by database router");
        }
        DatabaseRequest::GetNext {
            id,
            cycler_instance: received_cycler_instance,
            path,
            format,
        }
        | DatabaseRequest::Subscribe {
            id,
            cycler_instance: received_cycler_instance,
            path,
            format,
        } => {
            assert_eq!(cycler_instance, received_cycler_instance);
            if Database::exists(&path) {
                match subscriptions.entry((request.client.clone(), id)) {
                    Entry::Occupied(_) => {
                        let error_message = format!("already subscribed with id {id}");
                        let _ = request
                            .client
                            .response_sender
                            .send(Response::Textual(TextualResponse::Databases(
                                if is_get_next {
                                    TextualDatabaseResponse::GetNext {
                                        id,
                                        result: Err(error_message),
                                    }
                                } else {
                                    TextualDatabaseResponse::Subscribe {
                                        id,
                                        result: Err(error_message),
                                    }
                                },
                            )))
                            .await;
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(Subscription {
                            path,
                            format,
                            once: is_get_next,
                        });
                        if !is_get_next {
                            let _ = request
                                .client
                                .response_sender
                                .send(Response::Textual(TextualResponse::Databases(
                                    TextualDatabaseResponse::Subscribe { id, result: Ok(()) },
                                )))
                                .await;
                        }
                    }
                }
            } else {
                let _ = request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Databases(
                        TextualDatabaseResponse::Subscribe {
                            id,
                            result: Err(format!("path {path:?} does not exist")),
                        },
                    )))
                    .await;
            }
        }
        DatabaseRequest::Unsubscribe {
            id,
            subscription_id,
        } => {
            if subscriptions
                .remove(&(request.client.clone(), subscription_id))
                .is_none()
            {
                let _ = request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Databases(
                        TextualDatabaseResponse::Unsubscribe {
                            id,
                            result: Err(format!(
                                "never subscribed with subscription id {subscription_id}"
                            )),
                        },
                    )))
                    .await;
            } else {
                let _ = request
                    .client
                    .response_sender
                    .send(Response::Textual(TextualResponse::Databases(
                        TextualDatabaseResponse::Unsubscribe { id, result: Ok(()) },
                    )))
                    .await;
            }
        }
        DatabaseRequest::UnsubscribeEverything => {
            subscriptions
                .retain(|(client, _subscription_id), _subscription| &request.client != client);
        }
    }
}

async fn handle_notified_database(
    database_reader: &Reader<impl SerializeHierarchy>,
    subscriptions: &mut HashMap<(Client, usize), Subscription>,
) {
    let mut get_next_items = HashMap::new();
    let mut subscribed_items: HashMap<Client, HashMap<usize, Value>> = HashMap::new();
    {
        let database = database_reader.next();
        subscriptions.retain(|(client, subscription_id), subscription| {
            let data = match database.serialize_hierarchy(&subscription.path) {
                Ok(data) => data,
                Err(error) => {
                    error!("failed to serialize {:?}: {error:?}", subscription.path);
                    return true;
                }
            };
            if subscription.once {
                get_next_items.insert((client.clone(), *subscription_id), data);
                false
            } else {
                subscribed_items
                    .entry(client.clone())
                    .or_default()
                    .insert(*subscription_id, data);
                true
            }
        });
    }
    let send_results: Vec<_> = FuturesUnordered::from_iter(
        get_next_items
            .into_iter()
            .map(|((client, subscription_id), data)| {
                (
                    client.response_sender,
                    Response::Textual(TextualResponse::Databases(
                        TextualDatabaseResponse::GetNext {
                            id: subscription_id,
                            result: Ok(TextualDataOrBinaryReference::TextualData { data }),
                        },
                    )),
                )
            })
            .chain(subscribed_items.into_iter().map(|(client, items)| {
                (
                    client.response_sender,
                    Response::Textual(TextualResponse::Databases(
                        TextualDatabaseResponse::SubscribedData {
                            items: items
                                .into_iter()
                                .map(|(subscription_id, data)| {
                                    (
                                        subscription_id,
                                        TextualDataOrBinaryReference::TextualData { data },
                                    )
                                })
                                .collect(),
                        },
                    )),
                )
            }))
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

fn get_paths_from_hierarchy(prefix: String, hierarchy: HierarchyType) -> BTreeMap<String, String> {
    match hierarchy {
        HierarchyType::Primary { name } => [(prefix, name.to_string())].into(),
        HierarchyType::Struct { fields } => {
            let mut collected_fields = BTreeMap::new();
            if !prefix.is_empty() {
                collected_fields.insert(prefix.clone(), "GenericStruct".to_string());
            }
            for (name, nested_hierarchy) in fields {
                let prefix = if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}.{name}")
                };
                collected_fields.extend(get_paths_from_hierarchy(prefix, nested_hierarchy));
            }
            collected_fields
        }
        HierarchyType::GenericStruct => [(prefix, "GenericStruct".to_string())].into(),
        HierarchyType::GenericEnum => [(prefix, "GenericEnum".to_string())].into(),
        HierarchyType::Option { nested } => get_paths_from_hierarchy(prefix, *nested)
            .into_iter()
            .map(|(path, data_type)| (path, format!("Option<{data_type}>")))
            .collect(),
        HierarchyType::Vec { nested } => get_paths_from_hierarchy(prefix, *nested)
            .into_iter()
            .map(|(path, data_type)| (path, format!("Vec<{data_type}>")))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use color_eyre::{eyre::eyre, Result};
    use framework::multiple_buffer_with_slots;
    use serialize_hierarchy::HierarchyType;
    use tokio::{sync::mpsc::error::TryRecvError, task::yield_now, time::timeout};

    use crate::server::messages::Format;

    use super::*;

    struct DatabaseMock {
        existing_fields: HashMap<String, Value>,
    }

    impl SerializeHierarchy for DatabaseMock {
        fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
            self.existing_fields
                .get(field_path)
                .cloned()
                .ok_or_else(|| eyre!("missing"))
        }

        fn deserialize_hierarchy(&mut self, field_path: &str, data: Value) -> Result<()> {
            self.existing_fields.insert(field_path.to_string(), data);
            Ok(())
        }

        fn exists(field_path: &str) -> bool {
            field_path == "a.b.c"
        }

        fn get_hierarchy() -> HierarchyType {
            HierarchyType::Struct {
                fields: [(
                    "a".to_string(),
                    HierarchyType::Struct {
                        fields: [(
                            "b".to_string(),
                            HierarchyType::Struct {
                                fields: [(
                                    "c".to_string(),
                                    HierarchyType::Primary { name: "bool" },
                                )]
                                .into(),
                            },
                        )]
                        .into(),
                    },
                )]
                .into(),
            }
        }
    }

    async fn get_registered_request_sender_from_provider(
        cycler_instance: &'static str,
        database_changed: Arc<Notify>,
        database: Reader<impl SerializeHierarchy + Send + Sync + 'static>,
    ) -> (
        JoinHandle<()>,
        BTreeMap<String, String>,
        Sender<ClientRequest>,
    ) {
        let (databases_sender, mut databases_receiver) = channel(1);
        let join_handle = provider(
            databases_sender,
            cycler_instance,
            database_changed,
            database,
        );
        let (fields, request_sender) = timeout(Duration::from_secs(1), async move {
            let Some(request) = databases_receiver.recv().await else {
                panic!("expected request");
            };
            let Request::RegisterCycler { cycler_instance: cycler_instance_to_register, fields, request_sender } = request else {
                panic!("expected Request::RegisterCycler");
            };
            assert_eq!(cycler_instance, cycler_instance_to_register);
            assert!(databases_receiver.recv().await.is_none());
            (fields, request_sender)
        })
        .await
        .unwrap();
        (join_handle, fields, request_sender)
    }

    #[tokio::test]
    async fn provider_registers_itself_at_router() {
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            "CyclerInstance",
            database_changed,
            database_reader,
        )
        .await;

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn fields_are_collected() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: Default::default(),
        }]);

        let (provider_task, fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        assert_eq!(
            fields,
            [
                ("a".to_string(), "GenericStruct".to_string()),
                ("a.b".to_string(), "GenericStruct".to_string()),
                ("a.b.c".to_string(), "bool".to_string())
            ]
            .into()
        );

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_same_client_ids() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        const ID: usize = 42;
        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
                    id: ID,
                    cycler_instance,
                    path,
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_same_subscription_ids_and_different_client_ids() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        const ID: usize = 42;
        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
                    id: ID,
                    cycler_instance,
                    path,
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn subscriptions_with_different_subscription_ids_and_same_client_ids() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        let cycler_instance = cycler_instance.to_string();
        let path = "a.b.c".to_string();
        let format = Format::Textual;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
                    id: 1337,
                    cycler_instance,
                    path,
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_unknown_subscription_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_twice_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path: "a.b.c".to_string(),
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe_after_unsubscribe_everything_results_in_error() {
        let cycler_instance = "CyclerInstance";
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [("a.b.c".to_string(), 42.into())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed,
            database_reader,
        )
        .await;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
                    id: 42,
                    cycler_instance: cycler_instance.to_string(),
                    path: "a.b.c".to_string(),
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::UnsubscribeEverything,
                client: Client {
                    id: 1337,
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
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn data_from_notified_database_is_sent_to_subscribed_client() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed.clone(),
            database_reader,
        )
        .await;

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        database_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Databases(
                TextualDatabaseResponse::SubscribedData {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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

        database_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_subscriptions_duplicate_data() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed.clone(),
            database_reader,
        )
        .await;

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender0, mut response_receiver0) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        let (response_sender1, mut response_receiver1) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Subscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Subscribe {
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

        database_changed.notify_one();
        let subscribed_data = response_receiver0.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Databases(
                TextualDatabaseResponse::SubscribedData {
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
            Response::Textual(TextualResponse::Databases(
                TextualDatabaseResponse::SubscribedData {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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

        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::Unsubscribe {
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
                Response::Textual(TextualResponse::Databases(
                    TextualDatabaseResponse::Unsubscribe {
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

        database_changed.notify_one();
        match response_receiver0.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        match response_receiver1.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        provider_task.await.unwrap();
    }

    #[tokio::test]
    async fn get_next_forwards_data_once() {
        let cycler_instance = "CyclerInstance";
        let path = "a.b.c".to_string();
        let value = Value::from(42);
        let database_changed = Arc::new(Notify::new());
        let (_database_writer, database_reader) = multiple_buffer_with_slots([DatabaseMock {
            existing_fields: [(path.clone(), value.clone())].into(),
        }]);

        let (provider_task, _fields, request_sender) = get_registered_request_sender_from_provider(
            cycler_instance,
            database_changed.clone(),
            database_reader,
        )
        .await;

        const SUBSCRIPTION_ID: usize = 42;
        let client_id = 1337;

        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(ClientRequest {
                request: DatabaseRequest::GetNext {
                    id: SUBSCRIPTION_ID,
                    cycler_instance: cycler_instance.to_string(),
                    path,
                    format: Format::Textual,
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

        database_changed.notify_one();
        let subscribed_data = response_receiver.recv().await.unwrap();
        assert_eq!(
            subscribed_data,
            Response::Textual(TextualResponse::Databases(
                TextualDatabaseResponse::GetNext {
                    id: SUBSCRIPTION_ID,
                    result: Ok(TextualDataOrBinaryReference::TextualData { data: value })
                }
            )),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        database_changed.notify_one();
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }

        drop(request_sender);
        provider_task.await.unwrap();
    }
}
