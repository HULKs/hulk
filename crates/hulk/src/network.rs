use std::{
    io::Error,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::prelude::AsRawFd,
};

use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use libc::{
    c_void, close, epoll_create, epoll_ctl, epoll_event, epoll_wait, eventfd, write, EPOLLERR,
    EPOLLHUP, EPOLLIN, EPOLL_CTL_ADD,
};
use log::warn;
use parking_lot::Mutex;
use serde::Deserialize;
use types::hardware::{IncomingMessage, OutgoingMessage};

pub struct Network {
    parameters: Parameters,
    game_controller_state_socket: UdpSocket,
    spl_socket: UdpSocket,
    last_game_controller_address: Mutex<Option<SocketAddr>>,
    shutdown_file_descriptor: i32,
    polling_file_descriptor: i32,
    pending_messages_for_reading: Mutex<Vec<IncomingMessage>>,
}

macro_rules! wrap_unsafe {
    ($call:expr) => {
        match unsafe { $call } {
            -1 => Err(Error::last_os_error()),
            result => Ok(result),
        }
    };
}

impl Network {
    pub fn new(parameters: Parameters) -> Result<Self> {
        let polling_file_descriptor =
            wrap_unsafe!(epoll_create(3)).wrap_err("failed to create epoll file descriptor")?;
        let game_controller_state_socket =
            UdpSocket::bind(format!("[::]:{}", parameters.game_controller_state_port))
                .wrap_err("failed to bind to GameController state socket")?;
        let mut event = epoll_event {
            events: EPOLLIN as u32 | EPOLLERR as u32 | EPOLLHUP as u32,
            u64: game_controller_state_socket.as_raw_fd() as u64,
        };
        wrap_unsafe!(epoll_ctl(
            polling_file_descriptor,
            EPOLL_CTL_ADD,
            game_controller_state_socket.as_raw_fd(),
            &mut event
        ))
        .wrap_err("failed to add GameController state socket to epoll")?;
        let spl_socket = UdpSocket::bind(format!("[::]:{}", parameters.spl_port))
            .wrap_err("failed to bind to SPL socket")?;
        spl_socket
            .set_broadcast(true)
            .wrap_err("failed to enable broadcast on SPL socket")?;
        let mut event = epoll_event {
            events: EPOLLIN as u32 | EPOLLERR as u32 | EPOLLHUP as u32,
            u64: spl_socket.as_raw_fd() as u64,
        };
        wrap_unsafe!(epoll_ctl(
            polling_file_descriptor,
            EPOLL_CTL_ADD,
            spl_socket.as_raw_fd(),
            &mut event
        ))
        .wrap_err("failed to add SPL socket to epoll")?;
        let shutdown_file_descriptor =
            wrap_unsafe!(eventfd(0, 0)).wrap_err("failed to create eventfd for shutdown")?;
        let mut event = epoll_event {
            events: EPOLLIN as u32 | EPOLLERR as u32 | EPOLLHUP as u32,
            u64: shutdown_file_descriptor as u64,
        };
        wrap_unsafe!(epoll_ctl(
            polling_file_descriptor,
            EPOLL_CTL_ADD,
            shutdown_file_descriptor,
            &mut event
        ))
        .wrap_err("failed to add shutdown file descriptor to epoll")?;
        Ok(Self {
            parameters,
            game_controller_state_socket,
            spl_socket,
            last_game_controller_address: Mutex::new(None),
            shutdown_file_descriptor,
            polling_file_descriptor,
            pending_messages_for_reading: Mutex::new(vec![]),
        })
    }

    pub fn unblock_read(&self) -> Result<()> {
        let value = [1, 0, 0, 0, 0, 0, 0, 0];
        match unsafe {
            write(
                self.shutdown_file_descriptor,
                value.as_ptr() as *const c_void,
                value.len(),
            )
        } {
            -1 => Err(Error::last_os_error()).wrap_err("failed to write to eventfd for shutdown"),
            8 => Ok(()),
            bytes_written => bail!("unexpected bytes written to shutdown eventfd: {bytes_written}"),
        }
    }

    pub fn read(&self) -> Result<IncomingMessage> {
        {
            let mut pending_messages_for_reading = self.pending_messages_for_reading.lock();
            if !pending_messages_for_reading.is_empty() {
                return Ok(pending_messages_for_reading.swap_remove(0));
            }
        }

        Ok(loop {
            const MAXIMUM_NUMBER_OF_EVENTS: usize = 3;
            let mut events = [epoll_event { events: 0, u64: 0 }; MAXIMUM_NUMBER_OF_EVENTS];
            let number_of_events = wrap_unsafe!(epoll_wait(
                self.polling_file_descriptor,
                events.as_mut_ptr(),
                MAXIMUM_NUMBER_OF_EVENTS as i32,
                -1
            ))
            .wrap_err("failed to wait via epoll")?;
            if number_of_events <= 0 {
                bail!("expected at least one ready file descriptor");
            }

            let mut returned_message = None;
            for event in &events[0..number_of_events as usize] {
                let message = if event.u64 == self.game_controller_state_socket.as_raw_fd() as u64 {
                    let mut buffer = [0; 1024];
                    let (received_bytes, address) = self
                        .game_controller_state_socket
                        .recv_from(&mut buffer)
                        .wrap_err(
                            "failed to received UDP datagram from GameController state socket",
                        )?;
                    match buffer[0..received_bytes].try_into() {
                        Ok(parsed_message) => {
                            let mut last_game_controller_address =
                                self.last_game_controller_address.lock();
                            *last_game_controller_address = Some(address);
                            IncomingMessage::GameController(parsed_message)
                        }
                        Err(error) => {
                            warn!(
                            "Failed to parse GameController state message (will be discarded): {:?}",
                            error
                        );
                            continue;
                        }
                    }
                } else if event.u64 == self.spl_socket.as_raw_fd() as u64 {
                    let mut buffer = [0; 1024];
                    let (received_bytes, _address) = self
                        .spl_socket
                        .recv_from(&mut buffer)
                        .wrap_err("failed to received UDP datagram from SPL socket")?;
                    match buffer[0..received_bytes].try_into() {
                        Ok(parsed_message) => IncomingMessage::Spl(parsed_message),
                        Err(error) => {
                            warn!(
                                "Failed to parse SPL message (will be discarded): {:?}",
                                error
                            );
                            continue;
                        }
                    }
                } else if event.u64 == self.shutdown_file_descriptor as u64 {
                    bail!("termination requested");
                } else {
                    let data = event.u64;
                    bail!("unexpected epoll event data {data}");
                };
                if let None = returned_message {
                    returned_message = Some(message);
                } else {
                    self.pending_messages_for_reading.lock().push(message);
                }
            }
            if let Some(message) = returned_message {
                break message;
            }
        })
    }

    pub fn write(&self, message: OutgoingMessage) -> Result<()> {
        // TODO: send UDP datagrams asynchronously to not block write()
        match message {
            OutgoingMessage::GameController(message) => {
                let last_game_controller_address: Option<_> = {
                    let last_game_controller_address = self.last_game_controller_address.lock();
                    *last_game_controller_address
                };
                if let Some(last_game_controller_address) = last_game_controller_address {
                    let message: Vec<u8> = message.into();
                    self.game_controller_state_socket
                        .send_to(
                            message.as_slice(),
                            SocketAddr::new(
                                last_game_controller_address.ip(),
                                self.parameters.game_controller_return_port,
                            ),
                        )
                        .wrap_err("failed to send UDP datagram to GameController")?;
                }
            }
            OutgoingMessage::Spl(message) => {
                let message: Vec<u8> = message.into();
                self.game_controller_state_socket
                    .send_to(
                        message.as_slice(),
                        SocketAddr::new(Ipv4Addr::BROADCAST.into(), self.parameters.spl_port),
                    )
                    .wrap_err("failed to send UDP datagram to GameController")?;
            }
        }
        Ok(())
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        unsafe { close(self.polling_file_descriptor) };
        unsafe { close(self.shutdown_file_descriptor) };
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    game_controller_state_port: u16,
    game_controller_return_port: u16,
    spl_port: u16,
}
