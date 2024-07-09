use log::{error, info};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    task::JoinSet,
};
use tokio_tungstenite::accept_async;
use tokio_util::sync::CancellationToken;

use super::{connection::Connection, router::RouterHandle};

pub type ClientId = usize;

pub struct Acceptor {
    listener: TcpListener,
    cancellation_token: CancellationToken,
    router: RouterHandle,
    next_client_id: usize,
    connection_tasks: JoinSet<()>,
}

impl Acceptor {
    pub fn new(
        listener: TcpListener,
        router: RouterHandle,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            listener,
            cancellation_token,
            router,
            next_client_id: 0,
            connection_tasks: JoinSet::new(),
        }
    }

    pub async fn run(mut self) {
        info!(
            "Serving websocket connections on {}",
            self.listener.local_addr().unwrap()
        );
        loop {
            select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((socket, _)) => {
                            self.accept(socket).await;
                        }
                        Err(error) => {
                            error!("failed to accept incoming connection: {error}");
                        }
                    };
                },
                Some(result) = self.connection_tasks.join_next() => {
                    if let Err(error) = result {
                        error!("connection task failed: {error}");
                    }
                }
                () = self.cancellation_token.cancelled() => {
                    break
                },
            }
        }
        info!("stop accepting new clients, waiting for connections to finish...");
        while let Some(result) = self.connection_tasks.join_next().await {
            if let Err(error) = result {
                error!("connection task failed: {error}");
            }
        }
    }

    async fn accept(&mut self, socket: TcpStream) {
        let stream = match accept_async(socket).await {
            Ok(stream) => stream,
            Err(error) => {
                error!("failed to accept websocket connection: {error}");
                return;
            }
        };
        // TODO: keep the handle to shutdown the clients
        let (connection, _) = Connection::new(
            stream,
            self.next_client_id,
            self.router.clone(),
            self.cancellation_token.clone(),
        );
        self.next_client_id += 1;
        self.connection_tasks.spawn(connection.run());
    }
}
