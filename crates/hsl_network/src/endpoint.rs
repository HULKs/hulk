use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use log::warn;
use serde::Deserialize;
use thiserror::Error;
use tokio::{net::UdpSocket, select};
use types::messages::{IncomingMessage, OutgoingMessage};

pub struct Endpoint {
    ports: Ports,
    game_controller_state_socket: UdpSocket,
    hsl_socket: UdpSocket,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to bind socket")]
    CannotBind(io::Error),
    #[error("failed to enable broadcast socket option")]
    EnableBroadcast(io::Error),
    #[error("failed to read from socket")]
    ReadError(io::Error),
}

impl Endpoint {
    pub async fn new(parameters: Ports) -> Result<Self, Error> {
        let game_controller_state_socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            parameters.game_controller_state,
        ))
        .await
        .map_err(Error::CannotBind)?;
        let hsl_socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, parameters.hsl))
            .await
            .map_err(Error::CannotBind)?;
        hsl_socket
            .set_broadcast(true)
            .map_err(Error::EnableBroadcast)?;
        Ok(Self {
            ports: parameters,
            game_controller_state_socket,
            hsl_socket,
        })
    }

    pub async fn read(&self) -> Result<IncomingMessage, Error> {
        loop {
            let mut game_controller_state_buffer = [0; 1024];
            let mut hsl_buffer = [0; 1024];
            select! {
                result = self.game_controller_state_socket.recv_from(&mut game_controller_state_buffer) => {
                    let (received_bytes, address) = result.map_err(Error::ReadError)?;
                    match game_controller_state_buffer[0..received_bytes].try_into() {
                        Ok(parsed_message) => {
                            break Ok(IncomingMessage::GameController(address, parsed_message));
                        }
                        Err(error) => {
                            warn!("Failed to parse GameController state message (will be discarded): {error:?}");
                            continue;
                        }
                    }
                },
                result = self.hsl_socket.recv_from(&mut hsl_buffer) => {
                    let (received_bytes, _address) = result.map_err(Error::ReadError)?;
                    match bincode::deserialize(&hsl_buffer[0..received_bytes]) {
                        Ok(parsed_message) => {
                            break Ok(IncomingMessage::Hsl(parsed_message));
                        }
                        Err(error) => {
                            warn!("Failed to parse HSL message (will be discarded): {error:?}");
                            continue;
                        }
                    }
                }
            }
        }
    }

    pub async fn write(&self, message: OutgoingMessage) {
        match message {
            OutgoingMessage::GameController(destination, message) => {
                let message: Vec<u8> = message.into();
                self.send_game_controller_visual_referee_message(destination, message)
                    .await;
            }
            OutgoingMessage::Hsl(message) => match bincode::serialize(&message) {
                Ok(message) => {
                    if let Err(error) = self
                        .hsl_socket
                        .send_to(
                            message.as_slice(),
                            SocketAddr::new(Ipv4Addr::BROADCAST.into(), self.ports.hsl),
                        )
                        .await
                    {
                        warn!("Failed to send UDP datagram via HSL socket: {error:?}")
                    }
                }
                Err(error) => {
                    warn!("Failed to serialize Hulk Message: {error:?}")
                }
            },
        };
    }

    async fn send_game_controller_visual_referee_message(
        &self,
        destination: SocketAddr,
        message: Vec<u8>,
    ) {
        if let Err(error) = self
            .game_controller_state_socket
            .send_to(
                message.as_slice(),
                SocketAddr::new(destination.ip(), self.ports.game_controller_return),
            )
            .await
        {
            warn!("Failed to send UDP datagram to GameController: {error:?}")
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ports {
    game_controller_state: u16,
    game_controller_return: u16,
    hsl: u16,
}
