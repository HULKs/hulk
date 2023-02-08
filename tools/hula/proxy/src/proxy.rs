use std::{
    collections::HashMap,
    fs::remove_file,
    io::{BufWriter, ErrorKind, Read, Write},
    mem::size_of,
    os::unix::{
        io::AsRawFd,
        net::{UnixListener, UnixStream},
        prelude::RawFd,
    },
    ptr::read,
    slice::from_raw_parts,
    sync::{Arc, Mutex},
    thread::sleep,
    time::{Duration, Instant},
};

use color_eyre::eyre::{bail, Result, WrapErr};
use epoll::{ControlOptions, Event, Events};
use hula_types::{HulaControlFrame, RobotState};
use log::{debug, error, info, warn};
use rmp_serde::{encode::write_named, from_slice};

use crate::{
    idle::{charging_skull, send_idle},
    SharedState,
};
use constants::HULA_SOCKET_PATH;

const LOLA_SOCKET_PATH: &str = "/tmp/robocup";
const LOLA_SOCKET_RETRY_COUNT: usize = 60;
const LOLA_SOCKET_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const NO_EPOLL_TIMEOUT: i32 = -1;

fn wait_for_lola() -> Result<UnixStream> {
    for _ in 0..LOLA_SOCKET_RETRY_COUNT {
        if let Ok(socket) = UnixStream::connect(LOLA_SOCKET_PATH) {
            return Ok(socket);
        }
        info!("Waiting for LoLA socket to become available...");
        sleep(LOLA_SOCKET_RETRY_INTERVAL);
    }
    bail!("stopped after {} retries", LOLA_SOCKET_RETRY_COUNT)
}

pub struct Proxy {
    lola: UnixStream,
    hula: UnixListener,
    epoll_fd: RawFd,
    shared_state: Arc<Mutex<SharedState>>,
}

impl Proxy {
    pub fn initialize(shared_state: Arc<Mutex<SharedState>>) -> Result<Self> {
        let lola = wait_for_lola().wrap_err("failed to connect to LoLA")?;
        remove_file(HULA_SOCKET_PATH)
            .or_else(|error| match error.kind() {
                ErrorKind::NotFound => Ok(()),
                _ => Err(error),
            })
            .wrap_err("failed to unlink existing HuLA socket file")?;
        let hula = UnixListener::bind(HULA_SOCKET_PATH)
            .wrap_err_with(|| format!("failed to bind {HULA_SOCKET_PATH}"))?;

        let epoll_fd = epoll::create(false).wrap_err("failed to create epoll file descriptor")?;
        add_to_epoll(epoll_fd, lola.as_raw_fd())
            .wrap_err("failed to register LoLA file descriptor in epoll")?;
        add_to_epoll(epoll_fd, hula.as_raw_fd())
            .wrap_err("failed to register hula file descriptor in epoll")?;

        Ok(Self {
            lola,
            hula,
            epoll_fd,
            shared_state,
        })
    }

    pub fn run(mut self) -> Result<()> {
        let proxy_start = Instant::now();
        let mut connections = HashMap::new();
        let mut events = [Event::new(Events::empty(), 0); 16];
        let mut writer = BufWriter::with_capacity(786, self.lola.try_clone()?);

        debug!("Entering epoll loop...");
        loop {
            let number_of_events = epoll::wait(self.epoll_fd, NO_EPOLL_TIMEOUT, &mut events)
                .wrap_err("failed to wait for epoll")?;
            for event in &events[0..number_of_events] {
                let notified_fd = event.data as i32;
                if notified_fd == self.lola.as_raw_fd() {
                    handle_lola_event(
                        &mut self.lola,
                        &mut connections,
                        proxy_start,
                        &self.shared_state,
                    )?;
                } else if notified_fd == self.hula.as_raw_fd() {
                    register_connection(&mut self.hula, &mut connections, self.epoll_fd)?;
                } else {
                    handle_connection_event(
                        &mut connections,
                        notified_fd,
                        &mut writer,
                        &self.shared_state,
                    )?;
                }
            }

            if !connections
                .values()
                .any(|connection| connection.is_sending_control_frames)
            {
                send_idle(&mut writer, &self.shared_state).wrap_err(
                    "a shadowy flight into the dangerous world of a man who does not exist",
                )?;
            }
        }
    }
}

struct Connection {
    socket: UnixStream,
    is_sending_control_frames: bool,
}

fn handle_lola_event(
    lola: &mut UnixStream,
    connections: &mut HashMap<RawFd, Connection>,
    proxy_start: Instant,
    shared_state: &Arc<Mutex<SharedState>>,
) -> Result<()> {
    let since_start = proxy_start.elapsed();
    let mut robot_state = read_lola_message(lola).wrap_err("failed to read lola message")?;
    robot_state.received_at = since_start.as_secs_f32();
    {
        let mut shared_state = shared_state.lock().unwrap();
        shared_state.battery = Some(robot_state.battery);
        if shared_state.configuration.is_none() {
            shared_state.configuration = Some(robot_state.robot_configuration);
        }
    }

    if connections.is_empty() {
        return Ok(());
    }
    let state_storage_buffer = unsafe {
        from_raw_parts(
            &robot_state as *const RobotState as *const u8,
            size_of::<RobotState>(),
        )
    };
    connections.retain(|_, connection| {
        if let Err(error) = connection.socket.write_all(state_storage_buffer) {
            error!("Failed to write StateStorage to connection: {error}");
            return false;
        }
        if let Err(error) = connection.socket.flush() {
            error!("Failed to flush connection: {error}");
            return false;
        }
        true
    });
    Ok(())
}

fn read_lola_message(lola: &mut UnixStream) -> Result<RobotState> {
    let mut lola_data = [0; 896];
    lola.read_exact(&mut lola_data)
        .wrap_err("failed to read from LoLA socket")?;
    from_slice(&lola_data).wrap_err("failed to parse MessagePack from LoLA StateMessage")
}

fn register_connection(
    hula: &mut UnixListener,
    connections: &mut HashMap<RawFd, Connection>,
    poll_fd: RawFd,
) -> Result<()> {
    let (connection_stream, _) = hula.accept().wrap_err("failed to accept connection")?;
    let connection_fd = connection_stream.as_raw_fd();
    info!("Accepted connection with file descriptor {connection_fd}");
    if connections
        .insert(
            connection_fd,
            Connection {
                socket: connection_stream,
                is_sending_control_frames: false,
            },
        )
        .is_some()
    {
        panic!("connection is already registered");
    }
    add_to_epoll(poll_fd, connection_fd)
        .wrap_err("failed to register connection file descriptor")?;

    Ok(())
}

fn handle_connection_event(
    connections: &mut HashMap<RawFd, Connection>,
    notified_fd: RawFd,
    writer: &mut BufWriter<UnixStream>,
    shared_state: &Arc<Mutex<SharedState>>,
) -> Result<()> {
    match connections.get_mut(&notified_fd) {
        Some(connection) => {
            let mut read_buffer = [0; size_of::<HulaControlFrame>()];
            if let Err(error) = connection.socket.read_exact(&mut read_buffer) {
                error!("Failed to read from connection: {}", error);
                info!("Removing connection with file descriptor {}", notified_fd);
                // remove will drop, drop will close, close will EPOLL_CTL_DEL
                connections
                    .remove(&notified_fd)
                    .expect("connection file descriptor has to be registered");
                return Ok(());
            };
            let control_frame = unsafe { read(read_buffer.as_ptr() as *const HulaControlFrame) };
            let skull = match &shared_state.lock().unwrap().battery {
                Some(battery) => charging_skull(battery),
                _ => Default::default(),
            };
            let lola_message = control_frame.into_lola(skull);
            write_named(writer, &lola_message).wrap_err("failed to serialize control message")?;
            connection.is_sending_control_frames = true;
        }
        None => warn!(
            "Connection with file descriptor {} does not exist",
            notified_fd
        ),
    }
    Ok(())
}

fn add_to_epoll(
    poll_file_descriptor: RawFd,
    file_descriptor_to_add: RawFd,
) -> Result<(), systemd::Error> {
    epoll::ctl(
        poll_file_descriptor,
        ControlOptions::EPOLL_CTL_ADD,
        file_descriptor_to_add,
        Event::new(
            Events::EPOLLIN | Events::EPOLLERR | Events::EPOLLHUP,
            file_descriptor_to_add as u64,
        ),
    )
}
