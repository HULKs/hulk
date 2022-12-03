use std::{
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{Arc, Mutex},
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use log::debug;

use crate::{
    lola::{Battery, RobotConfiguration},
    service_manager::ServiceManager,
    termination::TerminationRequest,
};

const BEACON_INTERVAL: Duration = Duration::from_secs(1);
const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
const BEACON_PORT: u16 = 4242;

fn get_hulks_os_version() -> Result<String> {
    let os_release = ini::Ini::load_from_file("/etc/os-release")?;
    Ok(os_release
        .general_section()
        .get("VERSION_ID")
        .expect("Could not query VERSION_ID")
        .to_string())
}

pub fn start(
    termination_request: TerminationRequest,
    robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
    battery: Arc<Mutex<Option<Battery>>>,
) -> Result<JoinHandle<Result<()>>> {
    let hulks_os_version = get_hulks_os_version()?;
    let hostname = hostname::get()
        .wrap_err("failed to query hostname")?
        .to_str()
        .ok_or(eyre!("failed to decode hostname"))?
        .to_owned();
    let service_manager = ServiceManager::new()?;
    let thread = spawn(move || {
        while robot_configuration.lock().unwrap().is_none() || battery.lock().unwrap().is_none() {
            debug!("Waiting for robot configuration and battery...");
            sleep(Duration::from_secs(1));
        }
        let result = serve(
            termination_request.clone(),
            &service_manager,
            &hulks_os_version,
            &hostname,
            robot_configuration,
            battery,
        );
        termination_request.terminate();
        result
    });

    debug!("Starting aliveness thread...");
    Ok(thread)
}

fn serve(
    termination_request: TerminationRequest,
    service_manager: &ServiceManager,
    hulks_os_version: &str,
    hostname: &str,
    robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
    battery: Arc<Mutex<Option<Battery>>>,
) -> Result<()> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let mut buffer = [0; 100];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((num_bytes, peer)) => {
                println!("Received {num_bytes} bytes from {peer}");
            }
            Err(error) => {
                bail!("failed to receive from aliveness socket {error}");
            }
        }
    }
}
