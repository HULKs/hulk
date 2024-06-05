use std::path::Path;

use parameters::directory::{deserialize, serialize};
use path_serde::{PathDeserialize, PathSerialize};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{spawn, sync::mpsc::Receiver, task::JoinHandle};

use crate::{
    messages::{ParametersResponse, Response, TextualResponse},
    server::client::Client,
};

use super::StorageRequest;

pub fn storage<Parameters>(
    mut parameters_writer: buffered_watch::Sender<Parameters>,
    mut request_receiver: Receiver<StorageRequest>,
    parameters_directory: impl AsRef<Path> + Send + Sync + 'static,
    body_id: String,
    head_id: String,
) -> JoinHandle<()>
where
    Parameters: 'static
        + Clone
        + DeserializeOwned
        + PathDeserialize
        + PathSerialize
        + Send
        + Serialize
        + Sync,
{
    spawn(async move {
        let mut parameters = (*parameters_writer.borrow_mut()).clone();
        while let Some(request) = request_receiver.recv().await {
            handle_request(
                request,
                &mut parameters,
                &mut parameters_writer,
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
    parameters_writer: &mut buffered_watch::Sender<Parameters>,
    parameters_directory: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) where
    Parameters: Clone + DeserializeOwned + Serialize + PathSerialize + PathDeserialize,
{
    match request {
        StorageRequest::UpdateParameter {
            client,
            id,
            path,
            data,
        } => {
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
                let mut slot = parameters_writer.borrow_mut();
                *slot = parameters.clone();
            }

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
                let mut slot = parameters_writer.borrow_mut();
                *slot = parameters;
            }

            respond(
                client,
                ParametersResponse::LoadFromDisk { id, result: Ok(()) },
            )
            .await;
        }
        StorageRequest::StoreToDisk {
            client,
            id,
            scope,
            path,
        } => {
            if let Err(error) = serialize(
                parameters,
                scope,
                &path,
                parameters_directory,
                body_id,
                head_id,
            )
            .await
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
    use std::collections::HashMap;

    use path_serde::{deserialize, serialize, PathDeserialize};
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;
    use tokio::sync::mpsc::{channel, error::TryRecvError};

    use crate::server::client::Client;

    use super::*;

    #[tokio::test]
    async fn terminates_on_request_sender_drop() {
        let (parameters_writer, _parameters_reader) = buffered_watch::channel(42usize);
        let (request_sender, request_receiver) = channel(1);
        let subscriptions_task = storage(
            parameters_writer,
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

    impl<T> PathSerialize for ParametersFake<T>
    where
        T: Serialize,
    {
        fn serialize_path<S>(
            &self,
            path: &str,
            serializer: S,
        ) -> Result<S::Ok, serialize::Error<S::Error>>
        where
            S: Serializer,
        {
            self.existing_fields
                .get(path)
                .ok_or(serialize::Error::PathDoesNotExist {
                    path: path.to_string(),
                })?
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed)
        }
    }

    impl<T> PathDeserialize for ParametersFake<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        fn deserialize_path<'de, D>(
            &mut self,
            path: &str,
            deserializer: D,
        ) -> Result<(), deserialize::Error<D::Error>>
        where
            D: Deserializer<'de>,
        {
            self.existing_fields.insert(
                path.to_string(),
                T::deserialize(deserializer).map_err(deserialize::Error::DeserializationFailed)?,
            );
            Ok(())
        }
    }

    #[tokio::test]
    async fn update_request_writes_parameters_and_notifies() {
        let path = "a.b.c".to_string();
        let (parameters_writer, mut parameters_reader) = buffered_watch::channel(ParametersFake {
            existing_fields: [(path.clone(), 42)].into(),
        });
        let (request_sender, request_receiver) = channel(1);
        let subscriptions_task = storage(
            parameters_writer,
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
        parameters_reader.wait_for_change().await.unwrap();
        {
            let parameters = parameters_reader.borrow_and_mark_as_seen();
            assert_eq!(parameters.existing_fields.get(&path), Some(value).as_ref());
        }

        drop(request_sender);
        subscriptions_task.await.unwrap();
    }
}
