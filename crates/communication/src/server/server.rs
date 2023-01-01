use std::{collections::HashMap, io, sync::Arc, thread};

use serde::Serialize;
use tokio::{
    runtime::{self, Runtime},
    sync::{
        mpsc::{channel, Sender},
        oneshot, Mutex, Notify,
    },
};
use tokio_util::sync::CancellationToken;

use crate::server::databases::router::router;

use super::{
    acceptor::{acceptor, AcceptError},
    databases::Request,
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
    // cycler_database: Arc<Mutex<HashMap<String, Sender<DatabaseRequest>>>,
}

impl Server {
    pub fn start(keep_running: CancellationToken) -> Result<Self, StartError> {
        let (runtime_sender, runtime_receiver) = oneshot::channel();

        println!("Starting thread...");
        let join_handle = thread::Builder::new()
            .name("communication".to_string())
            .spawn(move || {
                println!("Starting runtime...");
                let runtime = match runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => Arc::new(runtime),
                    Err(error) => {
                        runtime_sender.send(None).expect(
                            "successful thread creation should always wait for runtime_sender",
                        );
                        return Err(StartError::RuntimeNotStarted(error));
                    }
                };

                println!("Starting initial task...");
                let inner_runtime = runtime.clone();
                runtime.block_on(async move {
                    println!("Sending runtime to parent...");
                    let (databases_sender, databases_receiver) = channel(1);
                    runtime_sender
                        .send(Some((inner_runtime, databases_sender.clone())))
                        .expect("successful thread creation should always wait for runtime_sender");

                    let acceptor_task = acceptor(keep_running.clone(), databases_sender);
                    let databases_task = router(databases_receiver);

                    keep_running.cancelled().await;

                    let acceptor_task_result = acceptor_task.await;
                    databases_task.await;

                    let mut task_errors = vec![];
                    if let Err(error) = acceptor_task_result.expect("failed to join acceptor task")
                    {
                        task_errors.push(StartError::AcceptError(error));
                    }

                    if task_errors.is_empty() {
                        Ok(())
                    } else {
                        Err(StartError::TasksErrored(task_errors))
                    }
                })
            })
            .map_err(|error| StartError::ThreadNotStarted(error))?;

        println!("Receiving runtime from task...");
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

        println!("Done with start");

        Ok(Self {
            runtime,
            databases_sender,
        })
    }

    // pub fn register_cycler_instance<Database>(
    //     &self,
    //     cycler_instance: &str,
    //     database_changed: Arc<Notify>,
    //     database: Reader<Database>,
    // ) {
    //     // spawn database subscription manager
    // }
}

struct Reader<T> {
    value: T,
}
