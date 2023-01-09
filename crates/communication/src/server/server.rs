use std::{collections::HashSet, io, sync::Arc, thread};

use framework::{Reader, Writer};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    runtime::{self, Runtime},
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
}

pub struct Server {
    runtime: Arc<Runtime>,
    outputs_sender: Sender<Request>,
}

impl Server {
    pub fn start(keep_running: CancellationToken) -> Result<Self, StartError> {
        let (runtime_sender, runtime_receiver) = oneshot::channel();

        let join_handle = thread::Builder::new()
            .name("communication".to_string())
            .spawn(move || {
                let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => Arc::new(runtime),
                    Err(error) => {
                        runtime_sender.send(None).expect(
                            "successful thread creation should always wait for runtime_sender",
                        );
                        return Err(StartError::RuntimeNotStarted(error));
                    }
                };

                let inner_runtime = runtime.clone();
                runtime.block_on(async move {
                    let (outputs_sender, outputs_receiver) = channel(1);
                    runtime_sender
                        .send(Some((inner_runtime, outputs_sender.clone())))
                        .expect("successful thread creation should always wait for runtime_sender");

                    let acceptor_task = acceptor(keep_running.clone(), outputs_sender);
                    let outputs_task = router(outputs_receiver);

                    keep_running.cancelled().await;

                    let acceptor_task_result = acceptor_task.await;
                    let outputs_task_result = outputs_task.await;

                    let mut task_errors = vec![];
                    if let Err(error) = acceptor_task_result.expect("failed to join acceptor task")
                    {
                        task_errors.push(StartError::AcceptError(error));
                    }
                    outputs_task_result.expect("failed to join outputs task");

                    if task_errors.is_empty() {
                        Ok(())
                    } else {
                        Err(StartError::TasksErrored(task_errors))
                    }
                })
            })
            .map_err(StartError::ThreadNotStarted)?;

        let (runtime, outputs_sender) = match runtime_receiver
            .blocking_recv()
            .expect("successful thread creation should always send into runtime_sender")
        {
            Some((runtime, outputs_sender)) => (runtime, outputs_sender),
            None => {
                return Err(join_handle
                    .join()
                    .expect("runtime thread cannot be joined")
                    .expect_err("runtime thread without runtime should return an error"));
            }
        };

        Ok(Self {
            runtime,
            outputs_sender,
        })
    }

    pub fn register_cycler_instance<Output>(
        &self,
        cycler_instance: &'static str,
        outputs_changed: Arc<Notify>,
        outputs_reader: Reader<Output>,
        subscribed_outputs_writer: Writer<HashSet<String>>,
    ) where
        Output: SerializeHierarchy + Send + Sync + 'static,
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
}
