use std::{
    collections::{hash_map::Entry, HashMap},
    fs::{remove_file, File},
    io::{BufWriter, Read, Write},
    mem::size_of,
    ops::{Deref, DerefMut},
    os::unix::{
        net::{UnixListener, UnixStream},
        prelude::{AsRawFd, FromRawFd, RawFd},
    },
    ptr::read,
    slice::from_raw_parts,
    sync::{Arc, Mutex},
    thread::{sleep, spawn, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Error, Result};
use byteorder::{ByteOrder, NativeEndian};
use epoll::{ControlOptions, Event, Events};
use log::{debug, error, info, warn};
use nix::sys::eventfd::{eventfd, EfdFlags};
use rmp_serde::{encode::write_named, from_read_ref};

use crate::{
    lola::{
        fill_red_eyes_into, Battery, ControlStorage, LoLAControlMessage, LoLAStateMessage,
        RobotConfiguration, StateStorage,
    },
    termination::TerminationRequest,
};

struct DroppingUnixListener {
    listener: UnixListener,
    path: &'static str,
}

impl DroppingUnixListener {
    fn bind(path: &'static str) -> Result<Self> {
        Ok(Self {
            listener: UnixListener::bind(path)?,
            path,
        })
    }
}

impl Deref for DroppingUnixListener {
    type Target = UnixListener;

    fn deref(&self) -> &Self::Target {
        &self.listener
    }
}

impl DerefMut for DroppingUnixListener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.listener
    }
}

impl Drop for DroppingUnixListener {
    fn drop(&mut self) {
        remove_file(self.path).expect("Failed to remove listening unix socket");
    }
}

pub struct Proxy {
    thread: Option<JoinHandle<()>>,
    shutdown_file: File,
    shutdown_file_descriptor: RawFd,
}

impl Proxy {
    const LISTENER_SOCKET_PATH: &'static str = "/tmp/hula";
    const LOLA_SOCKET_PATH: &'static str = "/tmp/robocup";
    const LOLA_SOCKET_RETRY_COUNT: usize = 60;
    const LOLA_SOCKET_RETRY_INTERVAL: Duration = Duration::from_secs(1);

    pub fn start(
        termination_request: TerminationRequest,
        robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
        battery: Arc<Mutex<Option<Battery>>>,
    ) -> Result<Self> {
        let shutdown_file_descriptor =
            eventfd(0, EfdFlags::empty()).context("Failed to open eventfd")?;
        let shutdown_file = unsafe { File::from_raw_fd(shutdown_file_descriptor) };

        let lola = {
            let mut result = None;
            for _ in 0..Self::LOLA_SOCKET_RETRY_COUNT {
                result = Some(UnixStream::connect(Self::LOLA_SOCKET_PATH));
                if result.as_ref().unwrap().is_ok() {
                    break;
                }
                if termination_request.is_requested() {
                    // this will return an error from this function and terminate the proxy construction
                    break;
                }
                info!("Waiting for LoLA socket to become available...");
                sleep(Self::LOLA_SOCKET_RETRY_INTERVAL);
            }
            result.unwrap()
        }
        .with_context(|| format!("Failed to connect to {}", Self::LOLA_SOCKET_PATH))?;
        let lola_file_descriptor = lola.as_raw_fd();
        let listener = DroppingUnixListener::bind(Self::LISTENER_SOCKET_PATH)
            .with_context(|| format!("Failed to bind {}", Self::LISTENER_SOCKET_PATH))?;
        let listener_file_descriptor = listener.as_raw_fd();

        let poll_file_descriptor =
            epoll::create(false).context("Failed to create epoll file descriptor")?;
        Self::add_to_epoll(poll_file_descriptor, shutdown_file_descriptor)
            .context("Failed to register shutdown file descriptor in epoll")?;
        Self::add_to_epoll(poll_file_descriptor, listener_file_descriptor)
            .context("Failed to register listener file descriptor in epoll")?;
        Self::add_to_epoll(poll_file_descriptor, lola_file_descriptor)
            .context("Failed to register LoLA file descriptor in epoll")?;

        let shutdown_file_for_thread = shutdown_file
            .try_clone()
            .context("Failed to duplicate shutdown file descriptor")?;
        let thread = spawn(move || {
            match Self::proxy_thread(
                robot_configuration,
                battery,
                shutdown_file_for_thread,
                shutdown_file_descriptor,
                lola,
                lola_file_descriptor,
                listener,
                listener_file_descriptor,
                poll_file_descriptor,
            ) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{:?}", error);
                    termination_request.terminate();
                }
            }
        });

        Ok(Self {
            thread: Some(thread),
            shutdown_file,
            shutdown_file_descriptor,
        })
    }

    pub fn join(mut self) -> Result<()> {
        let mut value = [0; 8];
        NativeEndian::write_u64(&mut value, 1);
        let write_result = self.shutdown_file.write(&value);
        let join_result = self.thread.take().unwrap().join();
        let close_result = epoll::close(self.shutdown_file_descriptor);

        let written_bytes = write_result.context("Failed to write to shutdown file")?;
        assert_eq!(written_bytes, 8);
        join_result.map_err(|error| anyhow!("Failed to join proxy thread: {:?}", error))?;
        close_result.context("Failed to close epoll file descriptor")?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn proxy_thread(
        robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
        battery: Arc<Mutex<Option<Battery>>>,
        shutdown_file: File,
        shutdown_file_descriptor: RawFd,
        mut lola: UnixStream,
        lola_file_descriptor: RawFd,
        mut listener: DroppingUnixListener,
        listener_file_descriptor: RawFd,
        poll_file_descriptor: i32,
    ) -> Result<()> {
        let start = Instant::now();
        let mut last_extraction_update = Instant::now();
        let mut connections = HashMap::new();
        let mut events = [Event::new(Events::empty(), 0); 16];
        let mut lola_data = [0; 896];
        let zero_control_storage = ControlStorage::default();
        let mut control_message = LoLAControlMessage::default();
        let mut writer = BufWriter::with_capacity(786, lola.try_clone()?);

        debug!("Entering proxy epoll loop...");
        'outer: loop {
            let number_of_events = epoll::wait(poll_file_descriptor, -1, &mut events)
                .context("Failed to wait for epoll")?;
            let now = SystemTime::now();
            let seconds = now.duration_since(UNIX_EPOCH).unwrap().as_secs_f64();

            for event in events.iter().take(number_of_events) {
                let event_file_descriptor = event.data;
                if event_file_descriptor == shutdown_file_descriptor as u64 {
                    Proxy::handle_shutdown_event(shutdown_file)?;
                    debug!("Got request to shutdown epoll loop...");
                    break 'outer;
                } else if event_file_descriptor == lola_file_descriptor as u64 {
                    Proxy::handle_lola_event(
                        &mut lola,
                        &mut lola_data,
                        &start,
                        &mut connections,
                        &mut last_extraction_update,
                        &robot_configuration,
                        &battery,
                    )?;
                } else if event_file_descriptor == listener_file_descriptor as u64 {
                    Proxy::handle_listener_event(
                        &mut listener,
                        &mut connections,
                        poll_file_descriptor,
                    )?;
                } else {
                    Proxy::handle_connection_event(
                        &mut connections,
                        event_file_descriptor,
                        &mut control_message,
                        seconds,
                        &battery,
                        &mut writer,
                    )?;
                }
            }

            if connections.is_empty() {
                zero_control_storage.fill_chest_into(&mut control_message);
                zero_control_storage.fill_ears_into(&mut control_message);
                fill_red_eyes_into(&seconds, &mut control_message);
                zero_control_storage.fill_foots_into(&mut control_message);
                zero_control_storage.fill_position_into(&mut control_message);
                zero_control_storage.fill_stiffness_into(&mut control_message);
                if let Some(battery) = *battery.lock().unwrap() {
                    battery.fill_into_skull(&seconds, &mut control_message);
                }
                write_named(&mut writer, &control_message)?;
            }
        }

        Ok(())
    }

    fn handle_shutdown_event(mut shutdown_file: File) -> Result<()> {
        let mut value = [0; 8];
        let read_bytes = shutdown_file
            .read(&mut value)
            .context("Failed to read from shutdown file descriptor")?;
        assert_eq!(read_bytes, 8);
        let value = NativeEndian::read_u64(&value);
        assert_eq!(value, 1);

        Ok(())
    }

    fn handle_lola_event(
        lola: &mut UnixStream,
        lola_data: &mut [u8; 896],
        start: &Instant,
        connections: &mut HashMap<RawFd, UnixStream>,
        last_extraction_update: &mut Instant,
        robot_configuration: &Arc<Mutex<Option<RobotConfiguration>>>,
        battery: &Arc<Mutex<Option<Battery>>>,
    ) -> Result<()> {
        lola.read_exact(lola_data)
            .context("Failed to read from LoLA")?;
        let received_at = start.elapsed();
        let state_message: LoLAStateMessage =
            from_read_ref(&lola_data).context("Failed to parse MessagePack from LoLA")?;
        if !connections.is_empty() {
            let state_storage = StateStorage::from(received_at, &state_message);
            let state_storage_buffer = unsafe {
                from_raw_parts(
                    &state_storage as *const StateStorage as *const u8,
                    size_of::<StateStorage>(),
                )
            };
            // retain will drop, drop will close, close will EPOLL_CTL_DEL
            connections.retain(|connection_file_descriptor, connection| {
                if let Err(error) = connection.write_all(state_storage_buffer) {
                    error!("Failed to write StateStorage to connection: {}", error);
                    info!(
                        "Removing connection with file descriptor {}",
                        connection_file_descriptor
                    );
                    return false;
                }
                if let Err(error) = connection.flush() {
                    error!("Failed to flush connection: {}", error);
                    info!(
                        "Removing connection with file descriptor {}",
                        connection_file_descriptor
                    );
                    return false;
                }

                true
            });
        }

        if last_extraction_update.elapsed() >= Duration::from_secs(1) {
            *last_extraction_update = Instant::now();
            {
                let mut locked_robot_configuration = robot_configuration.lock().unwrap();
                *locked_robot_configuration = Some(state_message.robot_configuration.into());
            }
            {
                let mut locked_battery = battery.lock().unwrap();
                *locked_battery = Some(state_message.battery.into());
            }
        }

        Ok(())
    }

    fn handle_listener_event(
        listener: &mut DroppingUnixListener,
        connections: &mut HashMap<RawFd, UnixStream>,
        poll_file_descriptor: RawFd,
    ) -> Result<()> {
        let (connection_stream, _) = listener.accept().context("Failed to accept connection")?;
        let connection_file_descriptor = connection_stream.as_raw_fd();
        info!(
            "Got new connection with file descriptor {}",
            connection_file_descriptor
        );
        let inserted = connections
            .insert(connection_file_descriptor, connection_stream)
            .is_none();
        assert!(inserted);
        Proxy::add_to_epoll(poll_file_descriptor, connection_file_descriptor)
            .context("Failed to register connection file descriptor")?;

        Ok(())
    }

    fn handle_connection_event(
        connections: &mut HashMap<RawFd, UnixStream>,
        event_file_descriptor: u64,
        control_message: &mut LoLAControlMessage,
        seconds: f64,
        battery: &Arc<Mutex<Option<Battery>>>,
        writer: &mut BufWriter<UnixStream>,
    ) -> Result<()> {
        match connections.entry(event_file_descriptor as i32) {
            Entry::Occupied(mut connection_stream) => {
                let mut read_buffer = [0; size_of::<ControlStorage>()];
                if let Err(error) = connection_stream.get_mut().read_exact(&mut read_buffer) {
                    error!("Failed to read from connection: {}", error);
                    info!(
                        "Removing connection with file descriptor {}",
                        event_file_descriptor
                    );
                    // remove will drop, drop will close, close will EPOLL_CTL_DEL
                    let got_removed = connections
                        .remove(&(event_file_descriptor as i32))
                        .is_some();
                    assert!(got_removed);
                    return Ok(());
                };
                let control_storage =
                    unsafe { read(read_buffer.as_ptr() as *const ControlStorage) };
                control_storage.fill_chest_into(control_message);
                control_storage.fill_ears_into(control_message);
                control_storage.fill_eyes_into(control_message);
                control_storage.fill_foots_into(control_message);
                control_storage.fill_position_into(control_message);
                control_storage.fill_stiffness_into(control_message);
                if let Some(battery) = *battery.lock().unwrap() {
                    battery.fill_into_skull(&seconds, control_message);
                }
                write_named(writer, control_message)?;
            }
            Entry::Vacant(_) => warn!(
                "Connection with file descriptor {} does not exist",
                event_file_descriptor
            ),
        }

        Ok(())
    }

    fn add_to_epoll(poll_file_descriptor: RawFd, file_descriptor_to_add: RawFd) -> Result<()> {
        epoll::ctl(
            poll_file_descriptor,
            ControlOptions::EPOLL_CTL_ADD,
            file_descriptor_to_add,
            Event::new(
                Events::EPOLLIN | Events::EPOLLERR | Events::EPOLLHUP,
                file_descriptor_to_add as u64,
            ),
        )
        .map_err(Error::from)
    }
}
