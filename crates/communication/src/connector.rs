use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use log::{error, info, warn};
use tokio::{net::TcpStream, spawn, sync::mpsc, task::JoinHandle, time::sleep};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{
    output_subscription_manager, parameter_subscription_manager,
    receiver::receiver as receiver_task, requester::requester, responder,
};

#[derive(Debug)]
pub enum Message {
    SetConnect(bool),
    SetAddress(String),
    ReconnectTimerElapsed,
    Connected(Box<WebSocketStream<MaybeTlsStream<TcpStream>>>),
    ConnectionFailed { info: String },
}

#[derive(Debug)]
enum ConnectionStatus {
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

pub async fn connector(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    output_subscription_manager: mpsc::Sender<output_subscription_manager::Message>,
    parameter_subscription_manager: mpsc::Sender<parameter_subscription_manager::Message>,
    responder: mpsc::Sender<responder::Message>,
    initial_address: Option<String>,
    initial_connect: bool,
) {
    let mut status = match (initial_address, initial_connect) {
        (Some(address), true) => {
            let ongoing_connection = spawn_connect(address.clone(), sender.clone());
            ConnectionStatus::Connecting {
                address,
                ongoing_connection,
            }
        }
        (address, connect) => ConnectionStatus::Disconnected { address, connect },
    };

    while let Some(message) = receiver.recv().await {
        status = match status {
            ConnectionStatus::Disconnected {
                connect: false,
                address: None,
            } => match message {
                Message::SetConnect(new_connect) => ConnectionStatus::Disconnected {
                    connect: new_connect,
                    address: None,
                },
                Message::SetAddress(new_address) => ConnectionStatus::Disconnected {
                    connect: false,
                    address: Some(new_address),
                },
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => panic!("This should never happen"),
            },
            ConnectionStatus::Disconnected {
                connect: false,
                address: Some(address),
            } => match message {
                Message::SetConnect(true) => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionStatus::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::SetConnect(false) => ConnectionStatus::Disconnected {
                    connect: false,
                    address: Some(address),
                },
                Message::SetAddress(new_address) => ConnectionStatus::Disconnected {
                    connect: false,
                    address: Some(new_address),
                },
                Message::Connected(_ws_stream) => {
                    warn!("Dropping connection, we do not want to connect anymore");
                    ConnectionStatus::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => ConnectionStatus::Disconnected {
                    connect: false,
                    address: Some(address),
                },
            },
            ConnectionStatus::Disconnected {
                connect: true,
                address: None,
            } => match message {
                Message::SetConnect(false) => ConnectionStatus::Disconnected {
                    connect: false,
                    address: None,
                },
                Message::SetConnect(true) => ConnectionStatus::Disconnected {
                    connect: true,
                    address: None,
                },
                Message::SetAddress(address) => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionStatus::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::Connected(_ws_stream) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
                Message::ReconnectTimerElapsed => panic!("This should never happen"),
            },
            ConnectionStatus::Disconnected {
                connect: true,
                address: Some(address),
            } => match message {
                Message::SetConnect(false) => ConnectionStatus::Disconnected {
                    connect: false,
                    address: Some(address),
                },
                Message::SetConnect(true) => ConnectionStatus::Disconnected {
                    connect: true,
                    address: Some(address),
                },
                Message::SetAddress(address) => ConnectionStatus::Disconnected {
                    connect: true,
                    address: Some(address),
                },
                Message::ReconnectTimerElapsed => {
                    let ongoing_connection = spawn_connect(address.clone(), sender.clone());
                    ConnectionStatus::Connecting {
                        address,
                        ongoing_connection,
                    }
                }
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { .. } => panic!("This should never happen"),
            },
            ConnectionStatus::Connecting {
                address,
                ongoing_connection,
            } => match message {
                Message::SetConnect(false) => {
                    ongoing_connection.abort();
                    ConnectionStatus::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::SetConnect(true) => ConnectionStatus::Connecting {
                    address,
                    ongoing_connection,
                },
                Message::SetAddress(new_address) => {
                    if new_address == address {
                        ConnectionStatus::Connecting {
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
                    let (requester_sender, requester_receiver) = mpsc::channel(10);
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
                    ConnectionStatus::Connected { address }
                }
                Message::ConnectionFailed { info } => {
                    error!("Connection failed: {}", info);
                    spawn_reconnect_timer(sender.clone());
                    ConnectionStatus::Disconnected {
                        connect: true,
                        address: Some(address),
                    }
                }
                Message::ReconnectTimerElapsed => ConnectionStatus::Connecting {
                    address,
                    ongoing_connection,
                },
            },
            ConnectionStatus::Connected { address } => match message {
                Message::SetConnect(false) => {
                    output_subscription_manager
                        .send(output_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    parameter_subscription_manager
                        .send(parameter_subscription_manager::Message::Disconnect)
                        .await
                        .unwrap();
                    ConnectionStatus::Disconnected {
                        connect: false,
                        address: Some(address),
                    }
                }
                Message::SetConnect(true) => ConnectionStatus::Connected { address },
                Message::SetAddress(new_address) => {
                    if new_address == address {
                        ConnectionStatus::Connected { address }
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
                        ConnectionStatus::Connecting {
                            address: new_address,
                            ongoing_connection,
                        }
                    }
                }
                Message::Connected(_) => panic!("This should never happen"),
                Message::ConnectionFailed { info } => {
                    error!("Connection failed: {}", info);
                    spawn_reconnect_timer(sender.clone());
                    ConnectionStatus::Disconnected {
                        connect: true,
                        address: Some(address),
                    }
                }
                Message::ReconnectTimerElapsed => ConnectionStatus::Connected { address },
            },
        };
    }
}

fn spawn_reconnect_timer(sender: mpsc::Sender<Message>) {
    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        sender.send(Message::ReconnectTimerElapsed).await.unwrap();
    });
}

fn spawn_connect(address: String, sender: mpsc::Sender<Message>) -> JoinHandle<()> {
    spawn(async move {
        match try_connect(address).await {
            Ok(ws_stream) => sender
                .send(Message::Connected(Box::new(ws_stream)))
                .await
                .unwrap(),
            Err(error) => sender
                .send(Message::ConnectionFailed {
                    info: format!("{:#}", error),
                })
                .await
                .unwrap(),
        };
    })
}

async fn try_connect(address: String) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    info!("Try connection to {}", address);
    let (ws_stream, _response) = tokio_tungstenite::connect_async(&address)
        .await
        .with_context(|| anyhow!("Cannot connect websocket to {address}"))?;
    Ok(ws_stream)
}

async fn replace_ongoing_connection(
    ongoing_connection: JoinHandle<()>,
    new_address: String,
    sender: mpsc::Sender<Message>,
) -> ConnectionStatus {
    ongoing_connection.abort();
    match ongoing_connection.await {
        Err(error) => {
            assert!(error.is_cancelled());
            let ongoing_connection = spawn_connect(new_address.clone(), sender);
            ConnectionStatus::Connecting {
                address: new_address,
                ongoing_connection,
            }
        }
        _ => panic!("Connection attempt was not cancelled. I don't know how to recover"),
    }
}
