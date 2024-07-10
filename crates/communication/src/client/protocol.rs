use std::{collections::HashMap, time::SystemTime};

use futures_util::StreamExt;
use log::{error, info, warn};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    net::TcpStream,
    select, spawn,
    sync::{mpsc, oneshot, watch},
    task::JoinSet,
};
use tokio_tungstenite::{
    tungstenite::{
        self,
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use crate::{
    messages::{
        Format, Path, Paths, Request, RequestId, RequestKind, Response, ResponseKind, TextOrBinary,
    },
    send_or_log::SendOrLogExt,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("connection closed")]
    Close,
    #[error(transparent)]
    Socket(#[from] tungstenite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
    #[error("server error: {0}")]
    Server(String),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug)]
pub enum SubscriptionEvent<T> {
    Successful { timestamp: SystemTime, value: T },
    Update { timestamp: SystemTime, value: T },
    Failure { error: Error },
}

impl From<Response> for SubscriptionEvent<Value> {
    fn from(response: Response) -> Self {
        match response.kind {
            Ok(ResponseKind::Subscribe {
                timestamp,
                value: TextOrBinary::Text(value),
            }) => Self::Successful { timestamp, value },
            Ok(ResponseKind::Update {
                timestamp,
                value: TextOrBinary::Text(value),
            }) => Self::Update { timestamp, value },
            Ok(response) => Self::Failure {
                error: Error::Protocol(format!(
                    "unexpected response in text subscription: {response:#?}",
                )),
            },
            Err(error) => Self::Failure {
                error: Error::Server(error),
            },
        }
    }
}

impl From<Response> for SubscriptionEvent<Vec<u8>> {
    fn from(response: Response) -> Self {
        match response.kind {
            Ok(ResponseKind::Subscribe {
                timestamp,
                value: TextOrBinary::Binary(value),
            }) => Self::Successful { timestamp, value },
            Ok(ResponseKind::Update {
                timestamp,
                value: TextOrBinary::Binary(value),
            }) => Self::Update { timestamp, value },
            Ok(response) => Self::Failure {
                error: Error::Protocol(format!(
                    "unexpected response in binary subscription: {response:#?}",
                )),
            },
            Err(error) => Self::Failure {
                error: Error::Server(error),
            },
        }
    }
}

enum Event {
    GetPaths {
        return_sender: oneshot::Sender<Result<Paths, Error>>,
    },
    ReadText {
        path: Path,
        return_sender: oneshot::Sender<Result<(SystemTime, Value), Error>>,
    },
    ReadBinary {
        path: Path,
        return_sender: oneshot::Sender<Result<(SystemTime, Vec<u8>), Error>>,
    },
    SubscribeText {
        path: Path,
        return_sender: oneshot::Sender<mpsc::Receiver<SubscriptionEvent<Value>>>,
    },
    SubscribeBinary {
        path: Path,
        return_sender: oneshot::Sender<mpsc::Receiver<SubscriptionEvent<Vec<u8>>>>,
    },
    Write {
        path: Path,
        value: TextOrBinary,
        return_sender: oneshot::Sender<Result<(), Error>>,
    },
}

#[derive(Debug, Clone)]
pub struct ProtocolHandle {
    sender: mpsc::Sender<Event>,
}

impl ProtocolHandle {
    pub async fn get_paths(&self) -> Result<Paths, Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self.sender.send(Event::GetPaths { return_sender }).await;
        return_receiver.await.map_err(|_| Error::Close)?
    }

    pub async fn read_text(&self, path: Path) -> Result<(SystemTime, Value), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self
            .sender
            .send(Event::ReadText {
                path,
                return_sender,
            })
            .await;
        return_receiver.await.map_err(|_| Error::Close)?
    }

    pub async fn read_binary(&self, path: Path) -> Result<(SystemTime, Vec<u8>), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self
            .sender
            .send(Event::ReadBinary {
                path,
                return_sender,
            })
            .await;
        return_receiver.await.map_err(|_| Error::Close)?
    }

    pub async fn subscribe_text(
        &self,
        path: Path,
    ) -> Result<mpsc::Receiver<SubscriptionEvent<Value>>, Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self
            .sender
            .send(Event::SubscribeText {
                path,
                return_sender,
            })
            .await;
        return_receiver.await.map_err(|_| Error::Close)
    }

    pub async fn subscribe_binary(
        &self,
        path: Path,
    ) -> Result<mpsc::Receiver<SubscriptionEvent<Vec<u8>>>, Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self
            .sender
            .send(Event::SubscribeBinary {
                path,
                return_sender,
            })
            .await;
        return_receiver.await.map_err(|_| Error::Close)
    }

    pub async fn write(&self, path: Path, value: TextOrBinary) -> Result<(), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        let _ = self
            .sender
            .send(Event::Write {
                path,
                value,
                return_sender,
            })
            .await;
        return_receiver.await.map_err(|_| Error::Close)?
    }
}

#[derive(Debug, Error)]
enum ClosingError {
    #[error("failed to serialize response")]
    JsonSerialization(#[source] serde_json::Error),
    #[error("failed to deserialize request")]
    JsonDeserialization(#[source] serde_json::Error),
    #[error("failed to deserialize request")]
    BincodeDeserialization(#[source] bincode::Error),
    #[error("connection no longer needed")]
    Finish,
}

impl ClosingError {
    fn into_close_frame(self) -> CloseFrame<'static> {
        match self {
            Self::JsonSerialization(error) => CloseFrame {
                code: CloseCode::Error,
                reason: error.to_string().into(),
            },
            Self::JsonDeserialization(error) => CloseFrame {
                code: CloseCode::Invalid,
                reason: error.to_string().into(),
            },
            Self::BincodeDeserialization(error) => CloseFrame {
                code: CloseCode::Invalid,
                reason: error.to_string().into(),
            },
            Self::Finish => CloseFrame {
                code: CloseCode::Normal,
                reason: "server shutting down".into(),
            },
        }
    }
}

pub struct Protocol {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    event_receiver: mpsc::Receiver<Event>,
    change_watch: watch::Sender<()>,
    next_request_id: RequestId,
    pending_requests: HashMap<RequestId, oneshot::Sender<Response>>,
    subscriptions: HashMap<RequestId, mpsc::Sender<Response>>,
    subscription_tasks: JoinSet<RequestId>,
}

impl Protocol {
    pub fn new(
        socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
        change_watch: watch::Sender<()>,
    ) -> (Self, ProtocolHandle) {
        let (event_sender, event_receiver) = mpsc::channel(1);
        let task = Self {
            socket,
            event_receiver,
            change_watch,
            next_request_id: 0,
            pending_requests: HashMap::new(),
            subscriptions: HashMap::new(),
            subscription_tasks: JoinSet::new(),
        };
        let handle = ProtocolHandle {
            sender: event_sender,
        };
        (task, handle)
    }

    pub async fn run(mut self) {
        let result = self.select_loop().await;
        if let Err(error) = result {
            info!("closing connection...");
            let close_frame = error.into_close_frame();
            self.socket
                .send_or_log(Message::Close(Some(close_frame)))
                .await;
            while (self.socket.next().await).is_some() {
                // wait for the server to close the connection
            }
        }
        info!("connection closed");
        // TODO: properly shut down the socket by sending a close frame
    }

    async fn select_loop(&mut self) -> Result<(), ClosingError> {
        loop {
            select! {
                maybe_event = self.event_receiver.recv() => {
                    match maybe_event {
                        Some(event) => self.handle_event(event).await?,
                        None => return Err(ClosingError::Finish)
                    }
                }
                maybe_message = self.socket.next() => {
                    match maybe_message {
                        Some(Ok(message)) => self.handle_message(message).await?,
                        Some(Err(error)) => {
                            error!("socket error: {error}");
                        }
                        None => return Ok(())
                    }
                }
                Some(maybe_unsubscription) = self.subscription_tasks.join_next() => {
                    let id = maybe_unsubscription.unwrap();
                    self.handle_unsubscription(id).await?;
                }
            };
            let _ = self.change_watch.send(());
        }
    }

    async fn handle_event(&mut self, event: Event) -> Result<(), ClosingError> {
        match event {
            Event::GetPaths { return_sender } => {
                let (response_sender, response_receiver) = oneshot::channel();
                self.request(RequestKind::GetPaths, response_sender).await?;
                spawn(wait_for_paths_response(response_receiver, return_sender));
            }
            Event::ReadText {
                path,
                return_sender,
            } => {
                let (response_sender, response_receiver) = oneshot::channel();
                self.request(
                    RequestKind::Read {
                        path,
                        format: Format::Text,
                    },
                    response_sender,
                )
                .await?;
                spawn(wait_for_read_text_response(
                    response_receiver,
                    return_sender,
                ));
            }
            Event::ReadBinary {
                path,
                return_sender,
            } => {
                let (response_sender, response_receiver) = oneshot::channel();
                self.request(
                    RequestKind::Read {
                        path,
                        format: Format::Binary,
                    },
                    response_sender,
                )
                .await?;
                spawn(wait_for_read_binary_response(
                    response_receiver,
                    return_sender,
                ));
            }
            Event::SubscribeText {
                path,
                return_sender,
            } => {
                let (response_sender, response_receiver) = mpsc::channel(1);
                let (update_sender, update_receiver) = mpsc::channel(1);
                info!("sending text subscription request for {path}");
                let id = self
                    .subscribe(path.clone(), Format::Text, response_sender)
                    .await?;
                self.subscription_tasks.spawn(serve_subscription(
                    response_receiver,
                    update_sender.clone(),
                    id,
                ));
                let _ = return_sender.send(update_receiver);
            }
            Event::SubscribeBinary {
                path,
                return_sender,
            } => {
                let (response_sender, response_receiver) = mpsc::channel(1);
                let (update_sender, update_receiver) = mpsc::channel(1);
                info!("sending binary subscription request for {path}...");
                let id = self
                    .subscribe(path.clone(), Format::Binary, response_sender)
                    .await?;
                self.subscription_tasks.spawn(serve_subscription(
                    response_receiver,
                    update_sender.clone(),
                    id,
                ));
                let _ = return_sender.send(update_receiver);
            }
            Event::Write {
                path,
                value,
                return_sender,
            } => {
                let (response_sender, response_receiver) = oneshot::channel();
                self.request(RequestKind::Write { path, value }, response_sender)
                    .await?;
                spawn(wait_for_write_response(response_receiver, return_sender));
            }
        }
        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<(), ClosingError> {
        let response: Response = match message {
            Message::Text(string) => {
                serde_json::from_str(&string).map_err(ClosingError::JsonDeserialization)?
            }
            Message::Binary(bytes) => {
                bincode::deserialize(&bytes).map_err(ClosingError::BincodeDeserialization)?
            }
            _ => return Ok(()),
        };
        if let Some(sender) = self.pending_requests.remove(&response.id) {
            let _ = sender.send(response);
            return Ok(());
        }
        if let Some(sender) = self.subscriptions.get(&response.id) {
            let _ = sender.send(response).await;
            return Ok(());
        }
        warn!("unexpected response");
        Ok(())
    }

    async fn handle_unsubscription(&mut self, id: RequestId) -> Result<(), ClosingError> {
        info!("unsubscribing from {id}");
        self.subscriptions.remove(&id).unwrap();
        let (response_sender, response_receiver) = oneshot::channel();
        self.request(RequestKind::Unsubscribe { id }, response_sender)
            .await?;
        spawn(wait_for_unsubscribe_response(response_receiver));
        Ok(())
    }

    async fn request(
        &mut self,
        request: RequestKind,
        response_sender: oneshot::Sender<Response>,
    ) -> Result<(), ClosingError> {
        let id = self.next_request_id;
        self.next_request_id += 1;
        let request = Request { id, kind: request };
        self.socket
            .send_or_log(Message::Text(
                serde_json::to_string(&request).map_err(ClosingError::JsonSerialization)?,
            ))
            .await;
        self.pending_requests.insert(id, response_sender);
        Ok(())
    }

    async fn subscribe(
        &mut self,
        path: Path,
        format: Format,
        response_sender: mpsc::Sender<Response>,
    ) -> Result<RequestId, ClosingError> {
        let id = self.next_request_id;
        self.next_request_id += 1;
        let request = Request {
            id,
            kind: RequestKind::Subscribe { path, format },
        };
        self.socket
            .send_or_log(Message::Text(
                serde_json::to_string(&request).map_err(ClosingError::JsonSerialization)?,
            ))
            .await;
        self.subscriptions.insert(id, response_sender);
        Ok(id)
    }
}

async fn wait_for_paths_response(
    response_receiver: oneshot::Receiver<Response>,
    return_sender: oneshot::Sender<Result<Paths, Error>>,
) {
    let Ok(response) = response_receiver.await else {
        return;
    };
    match response.kind {
        Ok(ResponseKind::Paths { paths }) => {
            let _ = return_sender.send(Ok(paths));
        }
        Ok(response) => {
            let _ = return_sender.send(Err(Error::Protocol(format!(
                "unexpected response: {response:#?}"
            ))));
        }
        Err(error) => {
            let _ = return_sender.send(Err(Error::Server(error)));
        }
    };
}

async fn wait_for_read_text_response(
    response_receiver: oneshot::Receiver<Response>,
    return_sender: oneshot::Sender<Result<(SystemTime, Value), Error>>,
) {
    let Ok(response) = response_receiver.await else {
        return;
    };
    match response.kind {
        Ok(ResponseKind::Read {
            timestamp,
            value: TextOrBinary::Text(value),
        }) => {
            let _ = return_sender.send(Ok((timestamp, value)));
        }
        Ok(response) => {
            let _ = return_sender.send(Err(Error::Protocol(format!(
                "unexpected response: {response:#?}"
            ))));
        }
        Err(error) => {
            let _ = return_sender.send(Err(Error::Server(error)));
        }
    };
}

async fn wait_for_read_binary_response(
    response_receiver: oneshot::Receiver<Response>,
    return_sender: oneshot::Sender<Result<(SystemTime, Vec<u8>), Error>>,
) {
    let Ok(response) = response_receiver.await else {
        return;
    };
    match response.kind {
        Ok(ResponseKind::Read {
            timestamp,
            value: TextOrBinary::Binary(value),
        }) => {
            let _ = return_sender.send(Ok((timestamp, value)));
        }
        Ok(response) => {
            let _ = return_sender.send(Err(Error::Protocol(format!(
                "unexpected response: {response:#?}"
            ))));
        }
        Err(error) => {
            let _ = return_sender.send(Err(Error::Server(error)));
        }
    };
}

async fn serve_subscription(
    mut response_receiver: mpsc::Receiver<Response>,
    update_sender: mpsc::Sender<impl From<Response>>,
    id: RequestId,
) -> RequestId {
    loop {
        select! {
            maybe_response = response_receiver.recv() => {
                match maybe_response {
                    Some(response) => {
                        let _ = update_sender.send(response.into()).await;
                    },
                    None => break,
                }
            }
            () = update_sender.closed() => {
                break
            }
        }
    }
    id
}

async fn wait_for_unsubscribe_response(response_receiver: oneshot::Receiver<Response>) {
    let Ok(response) = response_receiver.await else {
        return;
    };
    match response.kind {
        Ok(ResponseKind::Unsubscribe) => {}
        Ok(response) => {
            error!("unexpected response: {response:?}");
        }
        Err(error) => {
            error!("failed to unsubscribe: {error}");
        }
    };
}

async fn wait_for_write_response(
    response_receiver: oneshot::Receiver<Response>,
    return_sender: oneshot::Sender<Result<(), Error>>,
) {
    let Ok(response) = response_receiver.await else {
        return;
    };
    match response.kind {
        Ok(ResponseKind::Write) => {
            let _ = return_sender.send(Ok(()));
        }
        Ok(response) => {
            let _ = return_sender.send(Err(Error::Protocol(format!(
                "unexpected response: {response:#?}"
            ))));
        }
        Err(error) => {
            let _ = return_sender.send(Err(Error::Server(error)));
        }
    };
}
