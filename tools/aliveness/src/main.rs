use std::{
    collections::HashMap,
    mem::size_of,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ptr::read,
    sync::Arc,
};

use color_eyre::eyre::{bail, eyre, ContextCompat, Result, WrapErr};
use futures_util::stream::StreamExt;
use hula_types::{Battery, RobotState};
use serde::Serialize;
use service_manager::{ServiceManager, SystemServices};
use tokio::{
    io::AsyncReadExt,
    net::{UdpSocket, UnixStream},
    select,
    task::JoinHandle,
};
use zbus::{
    zvariant::{self, OwnedObjectPath},
    Connection, MatchRule, MessageStream, MessageType, Proxy,
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

struct MulticastSocket {
    interface_name: String,
    socket: Arc<UdpSocket>,
    dbus_conn: Connection,
}

impl MulticastSocket {
    async fn bind(interface_name: String) -> Result<(Arc<UdpSocket>, JoinHandle<Result<()>>)> {
        let dbus_conn = Connection::system().await?;

        let socket = Arc::new(
            UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
                .await
                .wrap_err("failed to bind beacon socket")?,
        );

        let interface = Self {
            interface_name,
            socket: socket.clone(),
            dbus_conn,
        };

        interface.join_multicast().await?;

        Ok((
            socket,
            tokio::spawn(async move { Self::listen_for_network_change(interface).await }),
        ))
    }

    async fn listen_for_network_change(self) -> Result<()> {
        let link_object = self.get_link_object().await?;

        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender("org.freedesktop.network1")?
            .path(link_object)?
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .build();

        let mut stream = MessageStream::for_match_rule(rule, &self.dbus_conn, Some(1)).await?;

        while let Some(Ok(msg)) = stream.next().await {
            if let Ok((_, data, _)) =
                msg.body::<(String, HashMap<String, zvariant::Value>, Vec<String>)>()
            {
                if let Some(zvariant::Value::Str(data)) = data.get("IPv4AddressState") {
                    match data.as_str() {
                        "routable" => {
                            println!("IPv4 Back online");
                            self.join_multicast().await?;
                        }
                        "off" => {
                            println!("IPv4 offline");
                        }
                        _ => (),
                    }
                }
            }
        }

        bail!("failed to next ")
    }
    async fn get_link_object(&self) -> Result<OwnedObjectPath> {
        // Get the link objects
        let proxy = Proxy::new(
            &self.dbus_conn,
            "org.freedesktop.network1",
            "/org/freedesktop/network1",
            "org.freedesktop.network1.Manager",
        )
        .await?;

        let links: Vec<(i32, String, OwnedObjectPath)> = proxy.call("ListLinks", &()).await?;

        links
            .into_iter()
            .find_map(|(_, interface_name, link_path)| {
                if interface_name == self.interface_name {
                    Some(link_path)
                } else {
                    None
                }
            })
            .context(format!(
                "No DBus path for interface {} found",
                self.interface_name
            ))
    }

    async fn get_ip(&self) -> Result<Option<Ipv4Addr>> {
        let link_object = self.get_link_object().await?;

        let proxy = Proxy::new(
            &self.dbus_conn,
            "org.freedesktop.network1",
            link_object,
            "org.freedesktop.network1.Link",
        )
        .await?;

        let description: String = proxy.call("Describe", &()).await?;
        let description: serde_json::Value = serde_json::from_str(&description)?;

        let address = description["Addresses"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|value| {
                if value["Family"].as_i64().unwrap() == 2 {
                    let address = &value["Address"];
                    Some(Ipv4Addr::new(
                        address[0].as_u64().unwrap() as u8,
                        address[1].as_u64().unwrap() as u8,
                        address[2].as_u64().unwrap() as u8,
                        address[3].as_u64().unwrap() as u8,
                    ))
                } else {
                    None
                }
            });

        Ok(address)
    }

    async fn join_multicast(&self) -> Result<()> {
        if let Some(ip) = self.get_ip().await? {
            println!("Joining multicast on {ip}");
            self.socket
                .join_multicast_v4(BEACON_MULTICAST_GROUP, ip)
                .wrap_err_with(|| format!("failed to join multicast group on {ip}"))?;
        }
        Ok(())
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
    // let socket = Arc::new(
    //     UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
    //         .await
    //         .wrap_err("failed to bind beacon socket")?,
    // );
    // socket
    //     .join_multicast_v4(BEACON_MULTICAST_GROUP, interface_ip)
    //     .wrap_err_with(|| format!("failed to join multicast group on {interface_ip}"))?;
    // join_multicast(socket.clone()).await?;
    let (socket, _handle) = MulticastSocket::bind("enp4s0".to_owned()).await?;
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
