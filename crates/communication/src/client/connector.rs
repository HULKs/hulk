use std::time::Duration;

use color_eyre::{eyre::WrapErr, Result};
use futures_util::StreamExt;
use log::{error, info, warn};
use tokio::{
    net::TcpStream,
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
    time::sleep,
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::client::{
    output_subscription_manager, parameter_subscription_manager,
    receiver::receiver as receiver_task, requester::requester, responder,
};

#[derive(Debug)]
pub enum Message {
    SubscribeToUpdates(Sender<ConnectionStatus>),
    SetConnect(bool),
    SetAddress(String),
    ReconnectTimerElapsed,
    Connected(Box<WebSocketStream<MaybeTlsStream<TcpStream>>>),
    ConnectionFailed { info: String },
}

#[derive(Debug)]
enum ConnectionState {
    Disconnected {
        address: Option<String>,
        connect: bool,
    },
    Connecting {
        address: String,
        ongoing_connection: JoinHandle<()>,
    },
    Connected {
        address: String,
    },
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected {
        address: Option<String>,
        connect: bool,
    },
    Connecting {
        address: String,
    },
    Connected {
        address: String,
    },
}

pub async fn connector(
    mut receiver: Receiver<Message>,
    sender: Sender<Message>,
    output_subscription_manager: Sender<output_subscription_manager::Message>,
    parameter_subscription_manager: Sender<parameter_subscription_manager::Message>,
    responder: Sender<responder::Message>,
    initial_address: Option<String>,
    initial_connect: bool,
) {
    let mut status = match (initial_address, initial_connect) {
        (Some(address), true) => {
            let ongoing_connection = spawn_connect(address.clone(), sender.clone());
            ConnectionState::Connecting {
                address,
                ongoing_connection,
            }
        }
        (address, connect) => ConnectionState::Disconnected { address, connect },
    };

    let mut subscribers = Vec::new();

    while let Some(message) = receiver.recv().await {
        status = match status {
            ConnectionState::Disconnected {
                connect: false,
                address: None,
            } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    status
                }
                Message::SetConnect(new_connect) => ConnectionState::Disconnected {
                    connect: new_connect,
                    address: None,
                },
                Message::SetAddress(new_address) => ConnectionState::Disconnected {
                    connect: false,
                    address: Some(new_address),
                },
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => panic!("This should never happen"),
            },
            ConnectionState::Disconnected {
                connect: false,
                address: Some(address),
            } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    ConnectionState::Disconnected {
                        address: Some(address),
                        connect: false,
                    }
                }
                Message::SetConnect(true) => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionState::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::SetConnect(false) => ConnectionState::Disconnected {
                    connect: false,
                    address: Some(address),
                },
                Message::SetAddress(new_address) => ConnectionState::Disconnected {
                    connect: false,
                    address: Some(new_address),
                },
                Message::Connected(_ws_stream) => {
                    warn!("Dropping connection, we do not want to connect anymore");
                    ConnectionState::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => ConnectionState::Disconnected {
                    connect: false,
                    address: Some(address),
                },
            },
            ConnectionState::Disconnected {
                connect: true,
                address: None,
            } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    status
                }
                Message::SetConnect(false) => ConnectionState::Disconnected {
                    connect: false,
                    address: None,
                },
                Message::SetConnect(true) => ConnectionState::Disconnected {
                    connect: true,
                    address: None,
                },
                Message::SetAddress(address) => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionState::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::Connected(_ws_stream) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => panic!("This should never happen"),
            },
            ConnectionState::Disconnected {
                connect: true,
                address: Some(address),
            } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    ConnectionState::Disconnected {
                        address: Some(address),
                        connect: true,
                    }
                }
                Message::SetConnect(false) => ConnectionState::Disconnected {
                    connect: false,
                    address: Some(address),
                },
                Message::SetConnect(true) => ConnectionState::Disconnected {
                    connect: true,
                    address: Some(address),
                },
                Message::SetAddress(address) => ConnectionState::Disconnected {
                    connect: true,
                    address: Some(address),
                },
                Message::ReconnectTimerElapsed => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionState::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
            },
            ConnectionState::Connecting {
                address,
                ongoing_connection,
            } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    ConnectionState::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::SetConnect(false) => {
                    ongoing_connection.abort();
                    ConnectionState::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::SetConnect(true) => ConnectionState::Connecting {
                    address,
                    ongoing_connection,
                },
                Message::SetAddress(new_address) => {
                    if new_address == address {
                        ConnectionState::Connecting {
                            address,
                            ongoing_connection,
                        }
                    } else {
                        replace_ongoing_connection(ongoing_connection, new_address, sender.clone())
                            .await
                    }
                }
                Message::Connected(ws_stream) => {
                    let (writer, reader) = (*ws_stream).split();
                    let (requester_sender, requester_receiver) = channel(10);
                    output_subscription_manager
                        .send(output_subscription_manager::Message::Connect {
                            requester: requester_sender.clone(),
                        })
                        .await
                        .unwrap();
                    parameter_subscription_manager
                        .send(parameter_subscription_manager::Message::Connect {
                            requester: requester_sender,
                        })
                        .await
                        .unwrap();
                    spawn(requester(requester_receiver, writer));
                    spawn(receiver_task(
                        reader,
                        responder.clone(),
                        output_subscription_manager.clone(),
                        parameter_subscription_manager.clone(),
                        sender.clone(),
                    ));
                    info!("Connected to {}", address);
                    ConnectionState::Connected { address }
                }
                Message::ConnectionFailed { info } => {
                    error!("Connection failed: {}", info);
                    spawn_reconnect_timer(sender.clone());
                    ConnectionState::Disconnected {
                        connect: true,
                        address: Some(address),
                    }
                }
                Message::ReconnectTimerElapsed => ConnectionState::Connecting {
                    address,
                    ongoing_connection,
                },
            },
            ConnectionState::Connected { address } => match message {
                Message::SubscribeToUpdates(sender) => {
                    subscribers.push(sender);
                    ConnectionState::Connected { address }
                }
                Message::SetConnect(false) => {
                    output_subscription_manager
                        .send(output_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    parameter_subscription_manager
                        .send(parameter_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    ConnectionState::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::SetConnect(true) => ConnectionState::Connected { address },
                Message::SetAddress(new_address) => {
                    if new_address == address {
                        ConnectionState::Connected { address }
                    } else {
                        output_subscription_manager
                            .send(output_subscription_manager::Message::Disconnect)
                            .await
                            .unwrap();
                        parameter_subscription_manager
                            .send(parameter_subscription_manager::Message::Disconnect)
                            .await
                            .unwrap();
                        let ongoing_connection = spawn_connect(new_address.clone(), sender.clone());
                        ConnectionState::Connecting {
                            address: new_address,
                            ongoing_connection,
                        }
                    }
                }
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { info } => {
                    error!("Connection failed: {}", info);
                    spawn_reconnect_timer(sender.clone());
                    output_subscription_manager
                        .send(output_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    parameter_subscription_manager
                        .send(parameter_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    ConnectionState::Disconnected {
                        connect: true,
                        address: Some(address),
                    }
                }
                Message::ReconnectTimerElapsed => ConnectionState::Connected { address },
            },
        };
        let status = match &status {
            ConnectionState::Disconnected { address, connect } => ConnectionStatus::Disconnected {
                address: address.clone(),
                connect: *connect,
            },
            ConnectionState::Connecting {
                address,
                ongoing_connection: _,
            } => ConnectionStatus::Connecting {
                address: address.to_string(),
            },
            ConnectionState::Connected { address } => ConnectionStatus::Connected {
                address: address.to_string(),
            },
        };
        subscribers.retain(|sender| sender.try_send(status.clone()).is_ok())
    }
}

fn spawn_reconnect_timer(sender: Sender<Message>) {
    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        sender.send(Message::ReconnectTimerElapsed).await.unwrap();
    });
}

fn spawn_connect(address: String, sender: Sender<Message>) -> JoinHandle<()> {
    spawn(async move {
        match try_connect(address).await {
            Ok(ws_stream) => sender
                .send(Message::Connected(Box::new(ws_stream)))
                .await
                .unwrap(),
            Err(error) => sender
                .send(Message::ConnectionFailed {
                    info: format!("{error:#}"),
                })
                .await
                .unwrap(),
        };
    })
}

async fn try_connect(address: String) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    info!("Try connection to {}", address);
    let (ws_stream, _response) = connect_async(&address)
        .await
        .wrap_err_with(|| format!("cannot connect websocket to {address}"))?;
    Ok(ws_stream)
}

async fn replace_ongoing_connection(
    ongoing_connection: JoinHandle<()>,
    new_address: String,
    sender: Sender<Message>,
) -> ConnectionState {
    ongoing_connection.abort();
    match ongoing_connection.await {
        Err(error) => {
            assert!(error.is_cancelled());
            let ongoing_connection = spawn_connect(new_address.clone(), sender);
            ConnectionState::Connecting {
                address: new_address,
                ongoing_connection,
            }
        }
        _ => panic!("Connection attempt was not cancelled. I don't know how to recover"),
    }
}
