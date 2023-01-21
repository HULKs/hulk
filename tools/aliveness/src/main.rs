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
use log::info;
use serde::Serialize;
use service_manager::{ServiceManager, SystemServices};
use tokio::{
    io::AsyncReadExt,
    net::{UdpSocket, UnixStream},
    select,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
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
    async fn initialize() -> Result<Self> {
        let hulks_os_version = load_hulks_os_version()
            .await
            .wrap_err("failed to load HULKs-OS version")?;
        let hostname = hostname::get()
            .wrap_err("failed to query hostname")?
            .into_string()
            .map_err(|hostname| eyre!("invalid utf8 in hostname: {hostname:?}"))?;

        Ok(Self {
            hulks_os_version,
            hostname,
            body_id: None,
            head_id: None,
            battery: None,
        })
    }
}

struct AlivenessService {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl AlivenessService {
    fn cancel(&self) {
        self.token.cancel();
    }

    async fn join(self) {
        self.handle.await.unwrap();
    }
}

async fn listen_for_network_change(
    interface_name: String,
    service_manager: Arc<ServiceManager>,
) -> Result<()> {
    let dbus_conn = Connection::system().await?;

    let link_object = get_link_object(&interface_name, &dbus_conn).await?;

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.network1")?
        .path(link_object)?
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .build();

    let mut stream = MessageStream::for_match_rule(rule, &dbus_conn, Some(1)).await?;

    let mut service = None;

    if let Some(ip) = get_ip(&interface_name, &dbus_conn).await? {
        service = Some(join_multicast(ip, interface_name.clone(), service_manager.clone()).await?);
    }

    while let Some(Ok(msg)) = stream.next().await {
        if let Ok((_, data, _)) =
            msg.body::<(String, HashMap<String, zvariant::Value>, Vec<String>)>()
        {
            if let Some(zvariant::Value::Str(data)) = data.get("IPv4AddressState") {
                match data.as_str() {
                    "routable" => {
                        info!("IPv4 on {} back online", interface_name);
                        let ip = get_ip(&interface_name, &dbus_conn)
                            .await?
                            .ok_or(eyre!("failed to get IP"))?;
                        service = Some(
                            join_multicast(ip, interface_name.clone(), service_manager.clone())
                                .await?,
                        );
                    }
                    "off" => {
                        info!("IPv4 on {} offline", interface_name);
                        if let Some(s) = service {
                            s.cancel();
                            s.join().await;
                            service = None;
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    bail!("failed to get next message")
}
async fn get_link_object(interface_name: &str, dbus_conn: &Connection) -> Result<OwnedObjectPath> {
    // Get the link objects
    let proxy = Proxy::new(
        dbus_conn,
        "org.freedesktop.network1",
        "/org/freedesktop/network1",
        "org.freedesktop.network1.Manager",
    )
    .await?;

    let links: Vec<(i32, String, OwnedObjectPath)> = proxy.call("ListLinks", &()).await?;

    links
        .into_iter()
        .find_map(|(_, name, link_path)| {
            if name == interface_name {
                Some(link_path)
            } else {
                None
            }
        })
        .context(format!(
            "No DBus path for interface {} found",
            interface_name
        ))
}

async fn get_ip(interface_name: &str, dbus_conn: &Connection) -> Result<Option<Ipv4Addr>> {
    let link_object = get_link_object(interface_name, dbus_conn).await?;

    let proxy = Proxy::new(
        dbus_conn,
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

async fn join_multicast(
    ip: Ipv4Addr,
    interface_name: String,
    service_manager: Arc<ServiceManager>,
) -> Result<AlivenessService> {
    let mut robot_info = RobotInfo::initialize().await?;

    let mut hula = UnixStream::connect(HULA_SOCKET_PATH)
        .await
        .wrap_err("failed to connect to HuLA socket")?;

    let socket = UdpSocket::bind(SocketAddrV4::new(BEACON_MULTICAST_GROUP, BEACON_PORT))
        .await
        .wrap_err("failed to bind beacon socket")?;
    socket
        .join_multicast_v4(BEACON_MULTICAST_GROUP, ip)
        .wrap_err_with(|| format!("failed to join multicast group on {}", ip))?;

    info!("Joined multicast on {}", ip);

    let token = CancellationToken::new();

    let handle = {
        let token = token.clone();

        tokio::spawn(async move {
            let mut buffer = [0; 1024];

            loop {
                select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    result = read_from_hula(&mut hula) => {
                        let robot_state = result.wrap_err("failed to read from hula").unwrap();
                        robot_info.body_id = Some(
                            String::from_utf8(robot_state.robot_configuration.body_id.to_vec())
                                .wrap_err("invalid utf8 in body_id").unwrap(),
                        );
                        robot_info.head_id = Some(
                            String::from_utf8(robot_state.robot_configuration.head_id.to_vec())
                                .wrap_err("invalid utf8 in head_id").unwrap(),
                        );
                        robot_info.battery = Some(robot_state.battery);
                    }
                    message = socket.recv_from(&mut buffer) => {
                        let (num_bytes, peer) = message.wrap_err("failed to read from beacon socket").unwrap();
                        handle_beacon(
                            &socket,
                            &interface_name,
                            &service_manager,
                            &robot_info,
                            &buffer[0..num_bytes],
                            peer,
                        )
                        .await.unwrap();
                    }
                }
            }
        })
    };

    Ok(AlivenessService { token, handle })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let service_manager = Arc::new(ServiceManager::connect().await?);

    let interface_name = "enp4s0".to_owned();

    listen_for_network_change(interface_name, service_manager).await
}

async fn handle_beacon(
    socket: &UdpSocket,
    interface_name: &str,
    service_manager: &Arc<ServiceManager>,
    robot_info: &RobotInfo,
    message: &[u8],
    peer: SocketAddr,
) -> Result<()> {
    if message != BEACON_HEADER {
        bail!("invalid beacon header {message:?}");
    }
    info!("Received beacon from {peer}");
    let system_services = SystemServices::query(service_manager).await?;
    let response = BeaconResponse {
        hostname: &robot_info.hostname,
        interface_name,
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
