use std::{io, sync::Arc, thread};

use framework::Reader;
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    runtime::{self, Runtime},
    sync::{
        mpsc::{channel, Sender},
        oneshot, Notify,
    },
};
use tokio_util::sync::CancellationToken;

use crate::server::databases::router::router;

use super::{
    acceptor::{acceptor, AcceptError},
    databases::{provider::provider, Request},
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
    databases_sender: Sender<Request>,
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
                    let (databases_sender, databases_receiver) = channel(1);
                    runtime_sender
                        .send(Some((inner_runtime, databases_sender.clone())))
                        .expect("successful thread creation should always wait for runtime_sender");

                    let acceptor_task = acceptor(keep_running.clone(), databases_sender);
                    let databases_task = router(databases_receiver);

                    keep_running.cancelled().await;

                    let acceptor_task_result = acceptor_task.await;
                    let databases_task_result = databases_task.await;

                    let mut task_errors = vec![];
                    if let Err(error) = acceptor_task_result.expect("failed to join acceptor task")
                    {
                        task_errors.push(StartError::AcceptError(error));
                    }
                    databases_task_result.expect("failed to join databases task");

                    if task_errors.is_empty() {
                        Ok(())
                    } else {
                        Err(StartError::TasksErrored(task_errors))
                    }
                })
            })
            .map_err(StartError::ThreadNotStarted)?;

        let (runtime, databases_sender) = match runtime_receiver
            .blocking_recv()
            .expect("successful thread creation should always send into runtime_sender")
        {
            Some((runtime, databases_sender)) => (runtime, databases_sender),
            None => {
                return Err(join_handle
                    .join()
                    .expect("runtime thread cannot be joined")
                    .expect_err("runtime thread without runtime should return an error"));
            }
        };


        Ok(Self {
            runtime,
            databases_sender,
        })
    }

    pub fn register_cycler_instance<Database>(
        &self,
        cycler_instance: &'static str,
        database_changed: Arc<Notify>,
        database_reader: Reader<Database>,
    ) where
        Database: SerializeHierarchy + Send + Sync + 'static,
    {
        let _guard = self.runtime.enter();
        provider(
            self.databases_sender.clone(),
            cycler_instance,
            database_changed,
            database_reader,
        );
    }
}
