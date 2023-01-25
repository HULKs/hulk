use std::{
    collections::HashSet,
    fmt::Debug,
    io,
    iter::repeat_with,
    path::Path,
    sync::Arc,
    thread::{self, JoinHandle},
};

use framework::{multiple_buffer_with_slots, Reader, Writer};
use serde::{de::DeserializeOwned, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    net::ToSocketAddrs,
    runtime::{self, Runtime as TokioRuntime},
    sync::{
        mpsc::{channel, Sender},
        oneshot, Notify,
    },
};
use tokio_util::sync::CancellationToken;

use crate::server::outputs::router::router;

use super::{
    acceptor::{acceptor, AcceptError},
    outputs::{provider::provider, Request},
    parameters::{
        directory::{deserialize, DirectoryError},
        storage::storage,
        subscriptions::subscriptions,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("error while accepting connections")]
    AcceptError(AcceptError),
    #[error("one or more tasks encountered an error")]
    TasksErrored(Vec<StartError>),
    #[error("thread not started")]
    ThreadNotStarted(io::Error),
    #[error("runtime not started")]
    RuntimeNotStarted(io::Error),
    #[error("initial parameters not parsed")]
    InitialParametersNotParsed(DirectoryError),
}

pub struct Runtime<Parameters> {
    join_handle: JoinHandle<Result<(), StartError>>,
    runtime: Arc<TokioRuntime>,
    outputs_sender: Sender<Request>,
    parameters_reader: Reader<Parameters>,
}

impl<Parameters> Runtime<Parameters>
where
    Parameters: Clone + DeserializeOwned + Send + Serialize + SerializeHierarchy + Sync + 'static,
{
    pub fn start(
        addresses: Option<impl ToSocketAddrs + Send + Sync + 'static>,
        parameters_directory: impl AsRef<Path> + Send + Sync + 'static,
        body_id: String,
        head_id: String,
        amount_of_parameters_slots: usize,
        keep_running: CancellationToken,
    ) -> Result<Self, StartError> {
        let (runtime_sender, runtime_receiver) = oneshot::channel();

        let join_handle = thread::Builder::new()
            .name("communication".to_string())
            .spawn(move || {
                let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => Arc::new(runtime),
                    Err(error) => {
                        runtime_sender.send(None).ok().expect(
                            "successful thread creation should always wait for runtime_sender",
                        );
                        return Err(StartError::RuntimeNotStarted(error));
                    }
                };

                let inner_runtime = runtime.clone();
                runtime.block_on(async move {
                    let initial_parameters: Parameters =
                        match deserialize(&parameters_directory, &body_id, &head_id).await {
                            Ok(initial_parameters) => initial_parameters,
                            Err(source) => {
                                runtime_sender.send(None).ok().expect(
                                "successful thread creation should always wait for runtime_sender",
                            );
                                return Err(StartError::InitialParametersNotParsed(source));
                            }
                        };

                    let (outputs_sender, outputs_receiver) = channel(1);

                    let (parameters_writer, parameters_reader) = multiple_buffer_with_slots(
                        repeat_with(|| initial_parameters.clone())
                            .take(amount_of_parameters_slots + 1),
                    );

                    let (parameters_sender, parameters_receiver) = channel(1);
                    let parameters_changed = Arc::new(Notify::new());
                    let (parameters_storage_sender, parameters_storage_receiver) = channel(1);

                    runtime_sender
                        .send(Some((
                            inner_runtime,
                            outputs_sender.clone(),
                            parameters_reader.clone(),
                        )))
                        .ok()
                        .expect("successful thread creation should always wait for runtime_sender");

                    // only start acceptor if addresses is Some
                    let acceptor_task = addresses.map(|addresses| {
                        acceptor(
                            addresses,
                            keep_running.clone(),
                            outputs_sender,
                            parameters_sender,
                        )
                    });
                    let outputs_task = router(outputs_receiver);
                    let parameters_subscriptions_task = subscriptions(
                        parameters_receiver,
                        parameters_reader,
                        parameters_changed.clone(),
                        parameters_storage_sender,
                    );
                    let parameters_storage_task = storage(
                        parameters_writer,
                        parameters_changed,
                        parameters_storage_receiver,
                        parameters_directory,
                        body_id,
                        head_id,
                    );

                    keep_running.cancelled().await;

                    let acceptor_task_result = match acceptor_task {
                        Some(acceptor_task) => Some(acceptor_task.await),
                        None => None,
                    };
                    let outputs_task_result = outputs_task.await;
                    let parameters_subscriptions_task_result = parameters_subscriptions_task.await;
                    let parameters_storage_task_result = parameters_storage_task.await;

                    let mut task_errors = vec![];
                    if let Some(acceptor_task_result) = acceptor_task_result {
                        if let Err(error) =
                            acceptor_task_result.expect("failed to join acceptor task")
                        {
                            task_errors.push(StartError::AcceptError(error));
                        }
                    }
                    outputs_task_result.expect("failed to join outputs task");
                    parameters_subscriptions_task_result.expect("failed to join outputs task");
                    parameters_storage_task_result.expect("failed to join outputs task");

                    if task_errors.is_empty() {
                        Ok(())
                    } else {
                        Err(StartError::TasksErrored(task_errors))
                    }
                })
            })
            .map_err(StartError::ThreadNotStarted)?;

        let (runtime, outputs_sender, parameters_reader) = match runtime_receiver
            .blocking_recv()
            .expect("successful thread creation should always send into runtime_sender")
        {
            Some((runtime, outputs_sender, parameters_reader)) => {
                (runtime, outputs_sender, parameters_reader)
            }
            None => {
                return Err(join_handle
                    .join()
                    .expect("failed to join runtime thread")
                    .expect_err("runtime thread without runtime should return an error"));
            }
        };

        Ok(Self {
            join_handle,
            runtime,
            outputs_sender,
            parameters_reader,
        })
    }

    pub fn join(self) -> thread::Result<Result<(), StartError>> {
        drop(self.outputs_sender);
        self.join_handle.join()
    }

    pub fn register_cycler_instance<Outputs>(
        &self,
        cycler_instance: &'static str,
        outputs_changed: Arc<Notify>,
        outputs_reader: Reader<Outputs>,
        subscribed_outputs_writer: Writer<HashSet<String>>,
    ) where
        Outputs: SerializeHierarchy + Send + Sync + 'static,
    {
        let _guard = self.runtime.enter();
        provider(
            self.outputs_sender.clone(),
            cycler_instance,
            outputs_changed,
            outputs_reader,
            subscribed_outputs_writer,
        );
    }

    pub fn get_parameters_reader(&self) -> Reader<Parameters> {
        self.parameters_reader.clone()
    }
}
