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
    spl_socket: UdpSocket,
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
        let spl_socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, parameters.spl))
            .await
            .map_err(Error::CannotBind)?;
        spl_socket
            .set_broadcast(true)
            .map_err(Error::EnableBroadcast)?;
        Ok(Self {
            ports: parameters,
            game_controller_state_socket,
            spl_socket,
        })
    }

    pub async fn read(&self) -> Result<IncomingMessage, Error> {
        loop {
            let mut game_controller_state_buffer = [0; 1024];
            let mut spl_buffer = [0; 1024];
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
                result = self.spl_socket.recv_from(&mut spl_buffer) => {
                    let (received_bytes, _address) = result.map_err(Error::ReadError)?;
                    match bincode::deserialize(&spl_buffer[0..received_bytes]) {
                        Ok(parsed_message) => {
                            break Ok(IncomingMessage::Spl(parsed_message));
                        }
                        Err(error) => {
                            warn!("Failed to parse SPL message (will be discarded): {error:?}");
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
            OutgoingMessage::Spl(message) => match bincode::serialize(&message) {
                Ok(message) => {
                    if let Err(error) = self
                        .spl_socket
                        .send_to(
                            message.as_slice(),
                            SocketAddr::new(Ipv4Addr::BROADCAST.into(), self.ports.spl),
                        )
                        .await
                    {
                        warn!("Failed to send UDP datagram via SPL socket: {error:?}")
                    }
                }
                Err(error) => {
                    warn!("Failed to serialize Hulk Message: {error:?}")
                }
            },
            OutgoingMessage::VisualReferee(destination, message) => {
                let message: Vec<u8> = message.into();
                self.send_game_controller_visual_referee_message(destination, message)
                    .await;
            }
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
    spl: u16,
}
