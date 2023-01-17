use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
};

use log::error;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    select, spawn,
    sync::mpsc::{unbounded_channel, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::messages::ParametersRequest;

use super::{
    client_request::ClientRequest,
    connection::{connection, ConnectionError},
    outputs,
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
    addresses: impl ToSocketAddrs + Send + Sync + 'static,
    keep_running: CancellationToken,
    outputs_sender: Sender<outputs::Request>,
    parameters_sender: Sender<ClientRequest<ParametersRequest>>,
) -> JoinHandle<Result<(), AcceptError>> {
    let next_client_id = AtomicUsize::default();
    spawn(async move {
        let (error_sender, mut error_receiver) = unbounded_channel();

        let listener = TcpListener::bind(addresses)
            .await
            .map_err(AcceptError::TcpListenerNotBound)?;

        loop {
            let (stream, _) = select! {
                result = listener.accept() => result.map_err(AcceptError::NotAccepted)?,
                _ = keep_running.cancelled() => break,
            };

            let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
            connection(
                stream,
                keep_running.clone(),
                error_sender.clone(),
                outputs_sender.clone(),
                parameters_sender.clone(),
                client_id,
            );
        }

        let mut connection_errors = vec![];
        while let Some(error) = error_receiver.recv().await {
            connection_errors.push(error);
        }

        if connection_errors.is_empty() {
            Ok(())
        } else {
            Err(AcceptError::ConnectionsErrored(connection_errors))
        }
    })
}
