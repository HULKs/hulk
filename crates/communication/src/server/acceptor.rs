use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
};

use log::error;
use tokio::{
    net::TcpListener,
    select, spawn,
    sync::mpsc::{unbounded_channel, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::{
    connection::{connection, ConnectionError},
    databases,
};

#[derive(Debug, thiserror::Error)]
pub enum AcceptError {
    #[error("failed to bind TCP listener")]
    TcpListenerNotBound(io::Error),
    #[error("failed to accept")]
    NotAccepted(io::Error),
    #[error("one or more connections encountered an error")]
    ConnectionsErrored(Vec<ConnectionError>),
}

pub fn acceptor(
    keep_running: CancellationToken,
    databases_sender: Sender<databases::Request>,
) -> JoinHandle<Result<(), AcceptError>> {
    let mut next_client_id = AtomicUsize::default();
    println!("acceptor started");
    spawn(async move {
        let (error_sender, mut error_receiver) = unbounded_channel();

        let listener = TcpListener::bind("[::]:1337")
            .await
            .map_err(|error| AcceptError::TcpListenerNotBound(error))?;

        println!("Entering accept loop...");
        loop {
            println!("Accepting...");
            let (stream, _) = select! {
                result = listener.accept() => result.map_err(|error| AcceptError::NotAccepted(error))?,
                _ = keep_running.cancelled() => break,
            };

            let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
            println!("Starting connection {client_id} {stream:?}...");
            connection(
                stream,
                keep_running.clone(),
                error_sender.clone(),
                databases_sender.clone(),
                client_id,
            );
        }

        let mut connection_errors = vec![];
        while let Some(error) = error_receiver.recv().await {
            println!("connection error: {error:?}");
            connection_errors.push(error);
        }

        if connection_errors.is_empty() {
            Ok(())
        } else {
            Err(AcceptError::ConnectionsErrored(connection_errors))
        }
    })
}
