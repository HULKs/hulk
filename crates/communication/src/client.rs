use std::{
    borrow::BorrowMut,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    sync::Arc,
    time::{Duration, SystemTime},
};

use log::{error, info};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    net::TcpStream,
    select, spawn,
    sync::{broadcast, mpsc, oneshot, watch},
    task::{JoinHandle, JoinSet},
    time::sleep,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use crate::{
    client::protocol::Protocol,
    messages::{Path, Paths, TextOrBinary},
    send_or_log::SendOrLogExt,
};

use self::protocol::{ProtocolHandle, SubscriptionEvent};

pub mod protocol;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error(transparent)]
    Protocol(#[from] protocol::Error),
    #[error("not connected")]
    NotConnected,
}

#[derive(Debug)]
pub struct SubscriptionHandle<T> {
    pub receiver: broadcast::Receiver<Arc<SubscriptionEvent<T>>>,
    _drop: mpsc::Sender<()>,
}
pub type JsonSubscriptionHandle = SubscriptionHandle<Value>;
pub type BinarySubscriptionHandle = SubscriptionHandle<Vec<u8>>;

#[derive(Debug)]
enum Event {
    Connect,
    Disconnect,
    SetAddress(String),
    ReadText {
        path: Path,
        return_sender: oneshot::Sender<Result<(SystemTime, Value), RequestError>>,
    },
    ReadBinary {
        path: Path,
        return_sender: oneshot::Sender<Result<(SystemTime, Vec<u8>), RequestError>>,
    },
    SubscribeText {
        path: Path,
        return_sender: oneshot::Sender<JsonSubscriptionHandle>,
    },
    SubscribeBinary {
        path: Path,
        return_sender: oneshot::Sender<BinarySubscriptionHandle>,
    },
    Write {
        path: Path,
        value: TextOrBinary,
        return_sender: oneshot::Sender<Result<(), RequestError>>,
    },
    GetStatus {
        return_sender: oneshot::Sender<Status>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug)]
enum State {
    Disconnected,
    Connecting {
        ongoing_connection: JoinHandle<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    },
    Connected {
        protocol_handle: ProtocolHandle,
        protocol_task: JoinHandle<()>,
    },
}

pub type PathsEvent = Arc<Option<Result<Paths, protocol::Error>>>;

#[derive(Clone, Debug)]
pub struct ConnectionHandle {
    sender: mpsc::Sender<Event>,
    change_watch: watch::Receiver<()>,
    pub paths: watch::Receiver<PathsEvent>,
}

impl ConnectionHandle {
    pub async fn connect(&self) {
        self.sender.send(Event::Connect).await.unwrap();
    }

    pub async fn disconnect(&self) {
        self.sender.send(Event::Disconnect).await.unwrap();
    }

    pub async fn set_address(&self, address: String) {
        self.sender.send(Event::SetAddress(address)).await.unwrap();
    }

    pub async fn read_text(
        &self,
        path: impl Into<Path>,
    ) -> Result<(SystemTime, Value), RequestError> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::ReadText {
                path: path.into(),
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn read_binary(
        &self,
        path: impl Into<Path>,
    ) -> Result<(SystemTime, Vec<u8>), RequestError> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::ReadBinary {
                path: path.into(),
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn subscribe_text(&self, path: impl Into<Path>) -> JsonSubscriptionHandle {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::SubscribeText {
                path: path.into(),
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn subscribe_binary(&self, path: impl Into<Path>) -> BinarySubscriptionHandle {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::SubscribeBinary {
                path: path.into(),
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn write(&self, path: Path, value: TextOrBinary) -> Result<(), RequestError> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::Write {
                path,
                value,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn status(&self) -> Status {
        let (return_sender, return_receiver) = oneshot::channel();
        self.sender
            .send(Event::GetStatus { return_sender })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub fn on_change(&self, callback: impl Fn() + Send + 'static) {
        let mut change_watch = self.change_watch.clone();
        spawn(async move {
            while change_watch.changed().await.is_ok() {
                callback();
            }
        });
    }
}

struct Subscription<T> {
    sender: broadcast::Sender<Arc<SubscriptionEvent<T>>>,
    drop: mpsc::WeakSender<()>,
    protocol_unsubscribe: Option<oneshot::Receiver<()>>,
}

pub struct Client {
    command_receiver: mpsc::Receiver<Event>,
    change_watch: watch::Sender<()>,
    connection_state: State,
    peer_address: String,
    paths_sender: watch::Sender<PathsEvent>,
    text_subscriptions: HashMap<Path, Subscription<Value>>,
    text_unsubscriptions: JoinSet<Path>,
    binary_subscriptions: HashMap<Path, Subscription<Vec<u8>>>,
    binary_unsubscriptions: JoinSet<Path>,
}

impl Client {
    pub fn new(peer_address: String) -> (Self, ConnectionHandle) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let (paths_sender, paths_receiver) = watch::channel(Arc::new(None));
        let (change_sender, change_receiver) = watch::channel(());

        let task = Self {
            command_receiver,
            change_watch: change_sender,
            connection_state: State::Disconnected,
            peer_address,
            paths_sender,
            text_subscriptions: HashMap::new(),
            text_unsubscriptions: JoinSet::new(),
            binary_subscriptions: HashMap::new(),
            binary_unsubscriptions: JoinSet::new(),
        };
        let handle = ConnectionHandle {
            sender: command_sender,
            paths: paths_receiver,
            change_watch: change_receiver,
        };
        (task, handle)
    }

    pub async fn run(mut self) {
        loop {
            match &mut self.connection_state {
                State::Disconnected => {
                    select! {
                        maybe_command = self.command_receiver.recv() => {
                            match maybe_command {
                                Some(command) => {
                                    self.handle_command(command).await;
                                },
                                None => break,
                            }
                        }
                        Some(path) = self.text_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.text_subscriptions.remove(&path);
                        }
                        Some(path) = self.binary_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.binary_subscriptions.remove(&path);
                        }
                    }
                }
                State::Connecting { ongoing_connection } => {
                    select! {
                        maybe_command = self.command_receiver.recv() => {
                            if let Some(command) = maybe_command {
                                self.handle_command(command).await;
                            } else {
                                ongoing_connection.abort();
                                break
                            }
                        }
                        maybe_socket = ongoing_connection.borrow_mut() => {
                            let socket = maybe_socket.unwrap();
                            self.handle_successful_connection(socket);
                        }
                        Some(path) = self.text_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.text_subscriptions.remove(&path);
                        }
                        Some(path) = self.binary_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.binary_subscriptions.remove(&path);
                        }
                    }
                }
                State::Connected { protocol_task, .. } => {
                    select! {
                        maybe_command = self.command_receiver.recv() => {
                            match maybe_command {
                                Some(command) => {
                                    self.handle_command(command).await;
                                },
                                None => break,
                            }
                        }
                        result = protocol_task => {
                            result.unwrap();
                            self.connection_state = State::Connecting {
                                ongoing_connection: spawn(try_connect(self.peer_address.clone()))
                            };
                        }
                        Some(path) = self.text_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.text_subscriptions.remove(&path);
                        }
                        Some(path) = self.binary_unsubscriptions.join_next() => {
                            let path = path.unwrap();
                            self.binary_subscriptions.remove(&path);
                        }
                    }
                }
            }
            let _ = self.change_watch.send(());
        }
        // TODO: properly shut down open tasks, like the protocol
    }

    async fn handle_command(&mut self, command: Event) {
        match command {
            Event::Connect => {
                if matches!(&self.connection_state, State::Disconnected) {
                    let ongoing_connection = spawn(try_connect(self.peer_address.clone()));
                    self.connection_state = State::Connecting { ongoing_connection };
                }
            }
            Event::Disconnect => match &mut self.connection_state {
                State::Disconnected => {}
                State::Connecting { ongoing_connection } => {
                    ongoing_connection.abort();
                    if let Ok(mut socket) = ongoing_connection.await {
                        let message = Message::Close(Some(CloseFrame {
                            code: CloseCode::Normal,
                            reason: "connection no longer needed".into(),
                        }));
                        socket.send_or_log(message).await;
                    }
                    self.connection_state = State::Disconnected;
                }
                State::Connected { .. } => {
                    self.connection_state = State::Disconnected;
                }
            },
            Event::SetAddress(address) => {
                self.peer_address = address;
                match &mut self.connection_state {
                    State::Disconnected => {}
                    State::Connecting { ongoing_connection } => {
                        ongoing_connection.abort();
                        self.connection_state = State::Connecting {
                            ongoing_connection: spawn(try_connect(self.peer_address.clone())),
                        };
                    }
                    State::Connected { .. } => {
                        self.connection_state = State::Connecting {
                            ongoing_connection: spawn(try_connect(self.peer_address.clone())),
                        };
                    }
                }
            }
            Event::ReadText {
                path,
                return_sender,
            } => {
                match &self.connection_state {
                    State::Disconnected | State::Connecting { .. } => {
                        let _ = return_sender.send(Err(RequestError::NotConnected));
                    }
                    State::Connected {
                        protocol_handle, ..
                    } => {
                        let protocol_handle = protocol_handle.clone();
                        spawn(async move {
                            let result = protocol_handle.read_text(path).await;
                            let _ = return_sender.send(result.map_err(RequestError::from));
                        });
                    }
                };
            }
            Event::ReadBinary {
                path,
                return_sender,
            } => {
                match &self.connection_state {
                    State::Disconnected | State::Connecting { .. } => {
                        let _ = return_sender.send(Err(RequestError::NotConnected));
                    }
                    State::Connected {
                        protocol_handle, ..
                    } => {
                        let protocol_handle = protocol_handle.clone();
                        spawn(async move {
                            let result = protocol_handle.read_binary(path).await;
                            let _ = return_sender.send(result.map_err(RequestError::from));
                        });
                    }
                };
            }
            Event::SubscribeText {
                path,
                return_sender,
            } => {
                let handle = self.subscribe_text(path).await;
                let _ = return_sender.send(handle);
            }
            Event::SubscribeBinary {
                path,
                return_sender,
            } => {
                let handle = self.subscribe_binary(path).await;
                let _ = return_sender.send(handle);
            }
            Event::Write {
                path,
                value,
                return_sender,
            } => {
                match &self.connection_state {
                    State::Disconnected | State::Connecting { .. } => {
                        let _ = return_sender.send(Err(RequestError::NotConnected));
                    }
                    State::Connected {
                        protocol_handle, ..
                    } => {
                        let protocol_handle = protocol_handle.clone();
                        spawn(async move {
                            let result = protocol_handle.write(path, value).await;
                            let _ = return_sender.send(result.map_err(RequestError::from));
                        });
                    }
                };
            }
            Event::GetStatus { return_sender } => {
                let status = match &self.connection_state {
                    State::Disconnected => Status::Disconnected,
                    State::Connecting { .. } => Status::Connecting,
                    State::Connected { .. } => Status::Connected,
                };
                let _ = return_sender.send(status);
            }
        }
    }

    fn handle_successful_connection(&mut self, socket: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        info!("connected to {address}", address = self.peer_address);

        let (protocol, handle) = Protocol::new(socket, self.change_watch.clone());
        let task = spawn(protocol.run());

        self.connection_state = State::Connected {
            protocol_handle: handle.clone(),
            protocol_task: task,
        };

        let paths_sender = self.paths_sender.clone();
        {
            let handle = handle.clone();
            spawn(async move {
                let _ = paths_sender.send(Arc::new(Some(handle.get_paths().await)));
            });
        }

        for (path, subscription) in &mut self.text_subscriptions {
            let handle = handle.clone();
            let path = path.clone();
            let update_sender = subscription.sender.clone();
            let (unsubscribe_sender, unsubscribe_receiver) = oneshot::channel();
            spawn(async move {
                if let Ok(protocol_receiver) = handle.subscribe_text(path).await {
                    spawn(serve_subscription(
                        protocol_receiver,
                        update_sender,
                        unsubscribe_sender,
                    ));
                }
            });
            subscription.protocol_unsubscribe = Some(unsubscribe_receiver);
        }

        for (path, subscription) in &mut self.binary_subscriptions {
            let handle = handle.clone();
            let path = path.clone();
            let update_sender = subscription.sender.clone();
            let (unsubscribe_sender, unsubscribe_receiver) = oneshot::channel();
            spawn(async move {
                if let Ok(protocol_receiver) = handle.subscribe_binary(path).await {
                    spawn(serve_subscription(
                        protocol_receiver,
                        update_sender,
                        unsubscribe_sender,
                    ));
                }
            });
            subscription.protocol_unsubscribe = Some(unsubscribe_receiver);
        }
    }

    async fn subscribe_text(&mut self, path: Path) -> SubscriptionHandle<Value> {
        match self.text_subscriptions.entry(path.clone()) {
            Occupied(mut entry) => {
                let subscription = entry.get();
                match subscription.drop.upgrade() {
                    Some(drop) => SubscriptionHandle {
                        receiver: subscription.sender.subscribe(),
                        _drop: drop,
                    },
                    None => {
                        let (update_sender, update_receiver) = broadcast::channel(10);
                        let (drop_sender, drop_receiver) = mpsc::channel(1);
                        let unsubscribe_receiver = if let State::Connected {
                            protocol_handle, ..
                        } = &self.connection_state
                        {
                            protocol_handle
                                .subscribe_text(path.clone())
                                .await
                                .map_or_else(
                                    |_| None,
                                    |protocol_receiver| {
                                        let (unsubscribe_sender, unsubscribe_receiver) =
                                            oneshot::channel();
                                        spawn(serve_subscription(
                                            protocol_receiver,
                                            update_sender.clone(),
                                            unsubscribe_sender,
                                        ));
                                        Some(unsubscribe_receiver)
                                    },
                                )
                        } else {
                            None
                        };
                        let subscription = Subscription {
                            sender: update_sender,
                            drop: drop_sender.downgrade(),
                            protocol_unsubscribe: unsubscribe_receiver,
                        };
                        self.text_unsubscriptions
                            .spawn(wait_for_unsubscription(drop_receiver, path));
                        entry.insert(subscription);
                        SubscriptionHandle {
                            receiver: update_receiver,
                            _drop: drop_sender,
                        }
                    }
                }
            }
            Vacant(entry) => {
                let (update_sender, update_receiver) = broadcast::channel(10);
                let (drop_sender, drop_receiver) = mpsc::channel(1);
                let unsubscribe_receiver = if let State::Connected {
                    protocol_handle, ..
                } = &self.connection_state
                {
                    protocol_handle
                        .subscribe_text(path.clone())
                        .await
                        .map_or_else(
                            |_| None,
                            |protocol_receiver| {
                                let (unsubscribe_sender, unsubscribe_receiver) = oneshot::channel();
                                spawn(serve_subscription(
                                    protocol_receiver,
                                    update_sender.clone(),
                                    unsubscribe_sender,
                                ));
                                Some(unsubscribe_receiver)
                            },
                        )
                } else {
                    None
                };
                let subscription = Subscription {
                    sender: update_sender,
                    drop: drop_sender.downgrade(),
                    protocol_unsubscribe: unsubscribe_receiver,
                };
                self.text_unsubscriptions
                    .spawn(wait_for_unsubscription(drop_receiver, path));
                entry.insert(subscription);
                SubscriptionHandle {
                    receiver: update_receiver,
                    _drop: drop_sender,
                }
            }
        }
    }

    async fn subscribe_binary(&mut self, path: Path) -> SubscriptionHandle<Vec<u8>> {
        match self.binary_subscriptions.entry(path.clone()) {
            Occupied(mut entry) => {
                let subscription = entry.get();
                match subscription.drop.upgrade() {
                    Some(drop) => SubscriptionHandle {
                        receiver: subscription.sender.subscribe(),
                        _drop: drop,
                    },
                    None => {
                        let (update_sender, update_receiver) = broadcast::channel(10);
                        let (drop_sender, drop_receiver) = mpsc::channel(1);
                        let unsubscribe_receiver = if let State::Connected {
                            protocol_handle, ..
                        } = &self.connection_state
                        {
                            protocol_handle
                                .subscribe_binary(path.clone())
                                .await
                                .map_or_else(
                                    |_| None,
                                    |protocol_receiver| {
                                        let (unsubscribe_sender, unsubscribe_receiver) =
                                            oneshot::channel();
                                        spawn(serve_subscription(
                                            protocol_receiver,
                                            update_sender.clone(),
                                            unsubscribe_sender,
                                        ));
                                        Some(unsubscribe_receiver)
                                    },
                                )
                        } else {
                            None
                        };
                        let subscription = Subscription {
                            sender: update_sender,
                            drop: drop_sender.downgrade(),
                            protocol_unsubscribe: unsubscribe_receiver,
                        };
                        self.binary_unsubscriptions
                            .spawn(wait_for_unsubscription(drop_receiver, path));
                        entry.insert(subscription);
                        SubscriptionHandle {
                            receiver: update_receiver,
                            _drop: drop_sender,
                        }
                    }
                }
            }
            Vacant(entry) => {
                let (update_sender, update_receiver) = broadcast::channel(10);
                let (drop_sender, drop_receiver) = mpsc::channel(1);
                let unsubscribe_receiver = if let State::Connected {
                    protocol_handle, ..
                } = &self.connection_state
                {
                    protocol_handle
                        .subscribe_binary(path.clone())
                        .await
                        .map_or_else(
                            |_| None,
                            |protocol_receiver| {
                                let (unsubscribe_sender, unsubscribe_receiver) = oneshot::channel();
                                spawn(serve_subscription(
                                    protocol_receiver,
                                    update_sender.clone(),
                                    unsubscribe_sender,
                                ));
                                Some(unsubscribe_receiver)
                            },
                        )
                } else {
                    None
                };
                let subscription = Subscription {
                    sender: update_sender,
                    drop: drop_sender.downgrade(),
                    protocol_unsubscribe: unsubscribe_receiver,
                };
                self.binary_unsubscriptions
                    .spawn(wait_for_unsubscription(drop_receiver, path));
                entry.insert(subscription);
                SubscriptionHandle {
                    receiver: update_receiver,
                    _drop: drop_sender,
                }
            }
        }
    }
}

async fn try_connect(address: String) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    info!("connecting to {address} ...");
    loop {
        match connect_async(&address).await {
            Ok((socket, _)) => {
                return socket;
            }
            Err(error) => {
                error!("failed to connect: {error}");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn serve_subscription<T>(
    mut protocol_receiver: mpsc::Receiver<SubscriptionEvent<T>>,
    update_sender: broadcast::Sender<Arc<SubscriptionEvent<T>>>,
    mut drop_sender: oneshot::Sender<()>,
) {
    loop {
        select! {
            maybe_response = protocol_receiver.recv() => {
                match maybe_response {
                    Some(response) => {
                        if update_sender.send(Arc::new(response)).is_err() {
                            break
                        };
                    },
                    None => break,
                }
            }
            () = drop_sender.closed() => {
                break
            }
        }
    }
}

async fn wait_for_unsubscription(mut drop_receiver: mpsc::Receiver<()>, path: Path) -> Path {
    while drop_receiver.recv().await.is_some() {}
    path
}
