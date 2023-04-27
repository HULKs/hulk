use std::{path::Path, sync::Arc};

use framework::Writer;
use parameters::directory::{deserialize, serialize};
use serde::{de::DeserializeOwned, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    spawn,
    sync::{mpsc::Receiver, Notify},
    task::JoinHandle,
};

use crate::{
    messages::{ParametersResponse, Response, TextualResponse},
    server::client::Client,
};

use super::StorageRequest;

pub fn storage<Parameters>(
    parameters_writer: Writer<Parameters>,
    parameters_changed: Arc<Notify>,
    mut request_receiver: Receiver<StorageRequest>,
    parameters_directory: impl AsRef<Path> + Send + Sync + 'static,
    body_id: String,
    head_id: String,
) -> JoinHandle<()>
where
    Parameters: Clone + DeserializeOwned + Send + Serialize + SerializeHierarchy + Sync + 'static,
{
    spawn(async move {
        let mut parameters = (*parameters_writer.next()).clone();
        while let Some(request) = request_receiver.recv().await {
            handle_request(
                request,
                &mut parameters,
                &parameters_writer,
                &parameters_changed,
                &parameters_directory,
                &body_id,
                &head_id,
            )
            .await;
        }
    })
}

async fn handle_request<Parameters>(
    request: StorageRequest,
    parameters: &mut Parameters,
    parameters_writer: &Writer<Parameters>,
    parameters_changed: &Arc<Notify>,
    parameters_directory: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) where
    Parameters: Clone + DeserializeOwned + Serialize + SerializeHierarchy,
{
    match request {
        StorageRequest::UpdateParameter {
            client,
            id,
            path,
            data,
        } => {
            if !Parameters::exists(&path) {
                respond(
                    client,
                    ParametersResponse::Update {
                        id,
                        result: Err(format!("path {path:?} does not exist")),
                    },
                )
                .await;
                return;
            }

            if let Err(error) = parameters.deserialize_path(&path, data) {
                respond(
                    client,
                    ParametersResponse::Update {
                        id,
                        result: Err(format!("failed to deserialize: {error:?}")),
                    },
                )
                .await;
                return;
            }

            {
                let mut slot = parameters_writer.next();
                *slot = parameters.clone();
            }
            parameters_changed.notify_one();

            respond(client, ParametersResponse::Update { id, result: Ok(()) }).await;
        }
        StorageRequest::LoadFromDisk { client, id } => {
            let parameters = match deserialize(parameters_directory, body_id, head_id).await {
                Ok(parameters) => parameters,
                Err(error) => {
                    respond(
                        client,
                        ParametersResponse::LoadFromDisk {
                            id,
                            result: Err(format!("failed to deserialize parameters: {error:?}")),
                        },
                    )
                    .await;
                    return;
                }
            };

            {
                let mut slot = parameters_writer.next();
                *slot = parameters;
            }
            parameters_changed.notify_one();

            respond(
                client,
                ParametersResponse::LoadFromDisk { id, result: Ok(()) },
            )
            .await;
        }
        StorageRequest::StoreToDisk { client, id } => {
            if let Err(error) = serialize(parameters, parameters_directory, body_id, head_id).await
            {
                respond(
                    client,
                    ParametersResponse::StoreToDisk {
                        id,
                        result: Err(format!("failed to serialize parameters: {error:?}")),
                    },
                )
                .await;
                return;
            }

            respond(
                client,
                ParametersResponse::StoreToDisk { id, result: Ok(()) },
            )
            .await;
        }
    }
}

async fn respond(client: Client, response: ParametersResponse) {
    client
        .response_sender
        .send(Response::Textual(TextualResponse::Parameters(response)))
        .await
        .expect("receiver should always wait for all senders");
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use framework::multiple_buffer_with_slots;
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;
    use serialize_hierarchy::Error;
    use tokio::sync::mpsc::{channel, error::TryRecvError};

    use crate::server::client::Client;

    use super::*;

    #[tokio::test]
    async fn terminates_on_request_sender_drop() {
        let (parameters_writer, _parameters_reader) = multiple_buffer_with_slots([42usize]);
        let parameters_changed = Arc::new(Notify::new());
        let (request_sender, request_receiver) = channel(1);
        let subscriptions_task = storage(
            parameters_writer,
            parameters_changed,
            request_receiver,
            ".",
            Default::default(),
            Default::default(),
        );

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }

    #[derive(Clone, Deserialize, Serialize)]
    struct ParametersFake<T> {
        existing_fields: HashMap<String, T>,
    }

    impl<T> SerializeHierarchy for ParametersFake<T>
    where
        T: DeserializeOwned + Serialize,
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

    #[tokio::test]
    async fn update_request_writes_parameters_and_notifies() {
        let path = "a.b.c".to_string();
        let (parameters_writer, parameters_reader) = multiple_buffer_with_slots([ParametersFake {
            existing_fields: [(path.clone(), 42)].into(),
        }]);
        let parameters_changed = Arc::new(Notify::new());
        let (request_sender, request_receiver) = channel(1);
        let subscriptions_task = storage(
            parameters_writer,
            parameters_changed.clone(),
            request_receiver,
            ".",
            Default::default(),
            Default::default(),
        );

        let value = 1337;
        let (response_sender, mut response_receiver) = channel(1);
        request_sender
            .send(StorageRequest::UpdateParameter {
                client: Client {
                    id: 1337,
                    response_sender: response_sender.clone(),
                },
                id: 42,
                path: path.clone(),
                data: Value::from(value),
            })
            .await
            .unwrap();
        let response = response_receiver.recv().await.unwrap();
        assert_eq!(
            response,
            Response::Textual(TextualResponse::Parameters(ParametersResponse::Update {
                id: 42,
                result: Ok(()),
            })),
        );
        match response_receiver.try_recv() {
            Err(TryRecvError::Empty) => {}
            response => panic!("unexpected result from try_recv(): {response:?}"),
        }
        let parameters = parameters_reader.next();
        assert_eq!(parameters.existing_fields.get(&path), Some(value).as_ref());
        parameters_changed.notified().await;

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }
}
