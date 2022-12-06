use std::{
    mem::size_of,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ptr::read,
};

use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use hula_types::{Battery, RobotState};
use serde::Serialize;
use service_manager::{ServiceManager, SystemServices};
use tokio::{
    io::AsyncReadExt,
    net::{UdpSocket, UnixStream},
    select,
};

mod service_manager;

const HULA_SOCKET_PATH: &str = "/tmp/hula";
const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
const BEACON_PORT: u16 = 4242;
const BEACON_HEADER: &[u8; 6] = b"BEACON";
const OS_RELEASE_PATH: &str = "/etc/os-release";

#[derive(Clone, Debug, Serialize)]
pub struct BeaconResponse<'a> {
    pub hostname: &'a str,
    pub interface_name: &'a str,
    pub system_services: SystemServices,
    pub hulks_os_version: &'a str,
    pub body_id: &'a Option<String>,
    pub head_id: &'a Option<String>,
    pub battery: Option<Battery>,
}

async fn load_hulks_os_version() -> Result<String> {
    let mut os_release = configparser::ini::Ini::new();
    os_release.load_async(OS_RELEASE_PATH).await.unwrap();
    os_release
        .get("default", "VERSION_ID")
        .ok_or_else(|| eyre!("no VERSION_ID in {OS_RELEASE_PATH}"))
}

async fn read_from_hula(stream: &mut UnixStream) -> Result<RobotState> {
    let mut read_buffer = [0; size_of::<RobotState>()];
    stream.read_exact(&mut read_buffer).await?;
    Ok(unsafe { read(read_buffer.as_ptr() as *const RobotState) })
}

struct RobotInfo {
    hulks_os_version: String,
    hostname: String,
    body_id: Option<String>,
    head_id: Option<String>,
    battery: Option<Battery>,
}

impl RobotInfo {
    fn new(hulks_os_version: String, hostname: String) -> Self {
        Self {
            hulks_os_version,
            hostname,
            body_id: None,
            head_id: None,
            battery: None,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let hulks_os_version = load_hulks_os_version()
        .await
        .wrap_err("failed to load HULKs-OS version")?;
    let hostname = hostname::get()
        .wrap_err("failed to query hostname")?
        .into_string()
        .map_err(|hostname| eyre!("invalid utf8 in hostname: {hostname:?}"))?;
    let mut hula = UnixStream::connect(HULA_SOCKET_PATH)
        .await
        .wrap_err("failed to connect to HuLA socket")?;
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
        .await
        .wrap_err("failed to bind beacon socket")?;
    let interface_ip = Ipv4Addr::new(10, 0, 24, 26);
    socket
        .join_multicast_v4(BEACON_MULTICAST_GROUP, interface_ip)
        .wrap_err_with(|| format!("failed to join multicast group on {interface_ip}"))?;
    let service_manager = ServiceManager::connect().await?;

    let mut robot_info = RobotInfo::new(hulks_os_version, hostname);
    let mut buffer = [0; 1024];
    loop {
        select! {
            result = read_from_hula(&mut hula) => {
                let robot_state = result.wrap_err("failed to read from hula")?;
                robot_info.body_id = Some(
                    String::from_utf8(robot_state.robot_configuration.body_id.to_vec())
                        .wrap_err("invalid utf8 in body_id")?,
                );
                robot_info.head_id = Some(
                    String::from_utf8(robot_state.robot_configuration.head_id.to_vec())
                        .wrap_err("invalid utf8 in head_id")?,
                );
                robot_info.battery = Some(robot_state.battery);
            }
            message = socket.recv_from(&mut buffer) => {
                let (num_bytes, peer) = message.wrap_err("failed to read from beacon socket")?;
                handle_beacon(
                    &socket,
                    &service_manager,
                    &robot_info,
                    &buffer[0..num_bytes],
                    peer,
                )
                .await?;
            }
        }
    }
}

async fn handle_beacon(
    socket: &UdpSocket,
    service_manager: &ServiceManager,
    robot_info: &RobotInfo,
    message: &[u8],
    peer: SocketAddr,
) -> Result<()> {
    if message != BEACON_HEADER {
        bail!("invalid beacon header {message:?}");
    }
    let system_services = SystemServices::query(service_manager).await?;
    let response = BeaconResponse {
        hostname: &robot_info.hostname,
        interface_name: "wlan0",
        system_services,
        hulks_os_version: &robot_info.hulks_os_version,
        body_id: &robot_info.body_id,
        head_id: &robot_info.head_id,
        battery: robot_info.battery,
    };
    let send_buffer = serde_json::to_vec(&response).wrap_err("failed to serialize response")?;
    socket
        .send_to(&send_buffer, peer)
        .await
        .wrap_err_with(|| format!("failed to send beacon response to peer at {peer}"))?;
    Ok(())
}
