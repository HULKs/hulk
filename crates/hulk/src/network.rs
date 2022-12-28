use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use log::warn;
use serde::Deserialize;
use tokio::{
    net::UdpSocket,
    runtime::{Builder, Runtime},
    select,
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;
use types::messages::{IncomingMessage, OutgoingMessage};

pub struct Network {
    parameters: Parameters,
    runtime: Runtime,
    keep_running: CancellationToken,
    game_controller_state_socket: Arc<UdpSocket>,
    spl_socket: Arc<UdpSocket>,
    last_game_controller_address: Arc<Mutex<Option<SocketAddr>>>,
}

impl Network {
    pub fn new(keep_running: CancellationToken, parameters: Parameters) -> Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .wrap_err("failed to create tokio runtime")?;
        let game_controller_state_socket = runtime
            .block_on(UdpSocket::bind(format!(
                "[::]:{}",
                parameters.game_controller_state_port
            )))
            .wrap_err("failed to bind to GameController state socket")?;
        let spl_socket = runtime
            .block_on(UdpSocket::bind(format!("[::]:{}", parameters.spl_port)))
            .wrap_err("failed to bind to SPL socket")?;
        spl_socket
            .set_broadcast(true)
            .wrap_err("failed to enable broadcast on SPL socket")?;
        Ok(Self {
            parameters,
            runtime,
            keep_running,
            game_controller_state_socket: Arc::new(game_controller_state_socket),
            spl_socket: Arc::new(spl_socket),
            last_game_controller_address: Arc::new(Mutex::new(None)),
        })
    }

    pub fn read(&self) -> Result<IncomingMessage> {
        self.runtime.block_on(async {
            loop {
                let mut game_controller_state_buffer = [0; 1024];
                let mut spl_buffer = [0; 1024];
                select! {
                    _ = self.keep_running.cancelled() => bail!("termination requested"),
                    result = self.game_controller_state_socket.recv_from(&mut game_controller_state_buffer) => {
                        let (received_bytes, address) = result
                            .wrap_err("failed to received UDP datagram from GameController state socket")?;
                        match game_controller_state_buffer[0..received_bytes].try_into() {
                            Ok(parsed_message) => {
                                let mut last_game_controller_address =
                                    self.last_game_controller_address.lock().await;
                                *last_game_controller_address = Some(address);
                                break Ok(IncomingMessage::GameController(parsed_message));
                            }
                            Err(error) => {
                                warn!(
                                    "Failed to parse GameController state message (will be discarded): {:?}",
                                    error
                                );
                                continue;
                            }
                        }
                    },
                    result = self.spl_socket.recv_from(&mut spl_buffer) => {
                        let (received_bytes, _address) = result
                            .wrap_err("failed to received UDP datagram from SPL socket")?;
                        match spl_buffer[0..received_bytes].try_into() {
                            Ok(parsed_message) => {
                                break Ok(IncomingMessage::Spl(parsed_message));
                            }
                            Err(error) => {
                                warn!(
                                    "Failed to parse SPL message (will be discarded): {:?}",
                                    error
                                );
                                continue;
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn write(&self, message: OutgoingMessage) -> Result<()> {
        self.runtime.spawn({
            let game_controller_return_port = self.parameters.game_controller_return_port;
            let spl_port = self.parameters.spl_port;
            let game_controller_state_socket = self.game_controller_state_socket.clone();
            let spl_socket = self.spl_socket.clone();
            let last_game_controller_address = self.last_game_controller_address.clone();
            async move {
                match message {
                    OutgoingMessage::GameController(message) => {
                        let last_game_controller_address: Option<_> = {
                            let last_game_controller_address =
                                last_game_controller_address.lock().await;
                            *last_game_controller_address
                        };
                        if let Some(last_game_controller_address) = last_game_controller_address {
                            let message: Vec<u8> = message.into();
                            match game_controller_state_socket
                                .send_to(
                                    message.as_slice(),
                                    SocketAddr::new(
                                        last_game_controller_address.ip(),
                                        game_controller_return_port,
                                    ),
                                )
                                .await
                            {
                                Ok(_) => {}
                                Err(error) => {
                                    warn!(
                                        "Failed to send UDP datagram to GameController: {error:?}"
                                    )
                                }
                            }
                        }
                    }
                    OutgoingMessage::Spl(message) => {
                        let message: Vec<u8> = message.into();
                        match spl_socket
                            .send_to(
                                message.as_slice(),
                                SocketAddr::new(Ipv4Addr::BROADCAST.into(), spl_port),
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(error) => {
                                warn!("Failed to send UDP datagram via SPL socket: {error:?}")
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    game_controller_state_port: u16,
    game_controller_return_port: u16,
    spl_port: u16,
}
