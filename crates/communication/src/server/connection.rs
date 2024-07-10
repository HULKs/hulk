use std::{collections::HashMap, time::SystemTime};

use color_eyre::eyre::{eyre, Report};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use thiserror::Error;
use tokio::{net::TcpStream, select, sync::mpsc};
use tokio_tungstenite::{
    tungstenite::{
        self,
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use crate::{
    messages::{Request, RequestId, RequestKind, Response, ResponseKind, TextOrBinary},
    send_or_log::SendOrLogExt,
};

use super::{
    acceptor::ClientId,
    router::RouterHandle,
    source::{SubscriptionHandle, Update},
};

#[derive(Debug, Error)]
enum ClosingError {
    #[error("failed to serialize response")]
    JsonSerialization(#[source] serde_json::Error),
    #[error("failed to serialize response")]
    BincodeSerialization(#[source] bincode::Error),
    #[error("failed to deserialize request")]
    JsonDeserialization(#[source] serde_json::Error),
    #[error("failed to deserialize request")]
    BincodeDeserialization(#[source] bincode::Error),
    #[error("server shutting down")]
    Shutdown,
}

impl ClosingError {
    fn into_close_frame(self) -> CloseFrame<'static> {
        let code = match &self {
            Self::JsonSerialization(_) => CloseCode::Error,
            Self::BincodeSerialization(_) => CloseCode::Error,
            Self::JsonDeserialization(_) => CloseCode::Invalid,
            Self::BincodeDeserialization(_) => CloseCode::Invalid,
            Self::Shutdown => CloseCode::Normal,
        };
        CloseFrame {
            code,
            reason: format!("{:#}", Report::from(self)).into(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    SendUpdate(Update),
}

#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    event_sender: mpsc::Sender<Event>,
    id: ClientId,
}

impl ConnectionHandle {
    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn try_send_update(&self, update: Update) {
        let _ = self.event_sender.try_send(Event::SendUpdate(update));
    }
}

pub struct Connection {
    subscriptions: HashMap<RequestId, SubscriptionHandle>,
    handle: ConnectionHandle,
    stream: WebSocketStream<TcpStream>,
    router: RouterHandle,
    event_receiver: mpsc::Receiver<Event>,
    server_cancellation: CancellationToken,
}

impl Connection {
    pub fn new(
        stream: WebSocketStream<TcpStream>,
        id: ClientId,
        router: RouterHandle,
        server_cancellation: CancellationToken,
    ) -> (Self, ConnectionHandle) {
        let (event_sender, event_receiver) = mpsc::channel(10);
        let handle = ConnectionHandle { event_sender, id };

        let task = Self {
            subscriptions: HashMap::new(),
            handle: handle.clone(),
            stream,
            router,
            event_receiver,
            server_cancellation,
        };
        (task, handle)
    }

    pub async fn run(mut self) {
        let result = self.serve_stream().await;
        if let Err(error) = result {
            let close_frame = error.into_close_frame();
            self.stream
                .send_or_log(Message::Close(Some(close_frame)))
                .await;
            while (self.stream.next().await).is_some() {
                // wait for the client to close the connection
            }
        }
        drop(self.handle);
        drop(self.subscriptions);
        while let Some(_event) = self.event_receiver.recv().await {
            // Drain the event receiver.
        }
        info!("connection closed");
    }

    async fn serve_stream(&mut self) -> Result<(), ClosingError> {
        loop {
            select! {
                maybe_event = self.event_receiver.recv() => {
                    let event = maybe_event.expect("we always hold a sender ourself");
                    self.handle_event(event).await?;
                }
                maybe_message = self.stream.next() => {
                    match maybe_message {
                        Some(message) => self.handle_message(message).await?,
                        None => return Ok(()),
                    }
                }
                () = self.server_cancellation.cancelled() => {
                    return Err(ClosingError::Shutdown);
                }
            }
        }
    }

    async fn handle_event(&mut self, event: Event) -> Result<(), ClosingError> {
        match event {
            Event::SendUpdate(update) => self.send_update(update).await,
        }
    }

    async fn handle_message(
        &mut self,
        message: Result<Message, tungstenite::Error>,
    ) -> Result<(), ClosingError> {
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                error!("websocket error: {error:#}");
                return Ok(());
            }
        };
        match message {
            Message::Text(string) => self.handle_text_request(string).await,
            Message::Binary(bytes) => self.handle_binary_request(bytes).await,
            _ => Ok(()),
        }
    }

    async fn handle_text_request(&mut self, string: String) -> Result<(), ClosingError> {
        let request: Request =
            serde_json::from_str(&string).map_err(ClosingError::JsonDeserialization)?;
        let id = request.id;
        let kind = self
            .handle_request(request)
            .await
            .map_err(|error| format!("{error:#}"));
        let response = Response { id, kind };
        let text = serde_json::to_string(&response).map_err(ClosingError::JsonSerialization)?;
        self.stream.send_or_log(Message::Text(text)).await;
        Ok(())
    }

    async fn handle_binary_request(&mut self, bytes: Vec<u8>) -> Result<(), ClosingError> {
        let request: Request =
            bincode::deserialize(&bytes).map_err(ClosingError::BincodeDeserialization)?;
        let id = request.id;
        let kind = self
            .handle_request(request)
            .await
            .map_err(|error| format!("{error:#}"));
        let response = Response { id, kind };
        let bytes = bincode::serialize(&response).map_err(ClosingError::BincodeSerialization)?;
        self.stream.send_or_log(Message::Binary(bytes)).await;
        Ok(())
    }

    async fn handle_request(&mut self, request: Request) -> Result<ResponseKind, Report> {
        match request.kind {
            RequestKind::GetPaths => {
                let paths = self.router.get_paths().await;
                Ok(ResponseKind::Paths { paths })
            }
            RequestKind::Read { path, format } => {
                let (timestamp, value) = self.router.read(path, format).await?;
                Ok(ResponseKind::Read { timestamp, value })
            }
            RequestKind::Subscribe { path, format } => {
                let (handle, timestamp, value) = self
                    .router
                    .subscribe(path, format, self.handle.clone(), request.id)
                    .await?;
                self.subscriptions.insert(request.id, handle);
                Ok(ResponseKind::Subscribe { timestamp, value })
            }
            RequestKind::Unsubscribe { id } => {
                let _handle = self
                    .subscriptions
                    .remove(&id)
                    .ok_or_else(|| eyre!("no subscription with id `{id}`"))?;
                Ok(ResponseKind::Unsubscribe)
            }
            RequestKind::Write { path, value } => {
                let timestamp = SystemTime::now();
                self.router.write(path, timestamp, value).await?;
                Ok(ResponseKind::Write)
            }
        }
    }

    async fn send_update(&mut self, update: Update) -> Result<(), ClosingError> {
        let messages = compose_update_messages(update)?;
        for message in messages {
            if let Err(error) = self.stream.feed(message).await {
                error!("failed to send update: {error:#}");
            }
        }
        if let Err(error) = self.stream.flush().await {
            error!("failed to flush updates: {error:#}");
        }
        Ok(())
    }
}

fn compose_update_messages(update: Update) -> Result<Vec<Message>, ClosingError> {
    let timestamp = update.timestamp;
    update
        .texts
        .into_iter()
        .map(|(id, value)| {
            let kind = value.map(|value| ResponseKind::Update {
                timestamp,
                value: TextOrBinary::Text(value),
            });
            let response = Response { id, kind };
            let string =
                serde_json::to_string(&response).map_err(ClosingError::JsonSerialization)?;
            Ok(Message::Text(string))
        })
        .chain(update.binaries.into_iter().map(|(id, value)| {
            let kind = value.map(|value| ResponseKind::Update {
                timestamp,
                value: TextOrBinary::Binary(value),
            });
            let response = Response { id, kind };
            let bytes =
                bincode::serialize(&response).map_err(ClosingError::BincodeSerialization)?;
            Ok(Message::Binary(bytes))
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        iter::once,
        time::{SystemTime, UNIX_EPOCH},
    };

    use path_serde::{PathIntrospect, PathSerialize};
    use serde::Serialize;
    use serde_json::json;

    use crate::{
        messages::{Format, Path},
        server::source::Source,
    };

    use super::*;

    #[derive(Debug, Clone, Serialize, PathSerialize, PathIntrospect)]
    struct Data {
        field: u32,
    }

    #[tokio::test]
    async fn subscribe() {
        let (mut data_sender, data_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Data { field: 42 }));
        let (subscriptions_sender, mut subscriptions_receiver) =
            buffered_watch::channel(HashSet::new());

        let (source, handle) = Source::new(data_receiver, subscriptions_sender);
        let task = tokio::spawn(source.run());

        let (client_event_sender, mut client_event_receiver) = mpsc::channel(1);
        let client = ConnectionHandle {
            event_sender: client_event_sender,
            id: 13,
        };

        let path = Path::from("field");
        let format = Format::Text;
        let id = 4;
        let (subscription, timestamp, value) = handle
            .subscribe(path, format, client.clone(), id)
            .await
            .unwrap();

        assert_eq!(timestamp, UNIX_EPOCH);
        assert_eq!(value, TextOrBinary::Text(json!(42)));

        assert_eq!(
            *subscriptions_receiver.borrow(),
            once(Path::from("field")).collect()
        );

        let timestamp = SystemTime::now();
        *data_sender.borrow_mut() = (timestamp, Data { field: 1337 });

        let update = Update {
            timestamp,
            texts: once((id, Ok(json!(1337)))).collect(),
            binaries: HashMap::new(),
        };
        let received_event = client_event_receiver.recv().await.unwrap();
        assert_eq!(received_event, Event::SendUpdate(update));

        drop(subscription);

        *data_sender.borrow_mut() = (UNIX_EPOCH, Data { field: 42 });

        // The subscription is dropped, so the client should not receive any updates.
        assert!(client_event_receiver.try_recv().is_err());

        drop(handle);
        task.await.unwrap();
    }
}
