use std::{
    collections::HashMap,
    env::args,
    mem::size_of,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ptr::read,
    time::Duration,
};

use aliveness::{
    service_manager::SystemServices, AlivenessState, BEACON_HEADER, BEACON_MULTICAST_GROUP,
    BEACON_PORT,
};
use color_eyre::eyre::{bail, eyre, ContextCompat, Result, WrapErr};
use configparser::ini::Ini;
use futures_util::stream::StreamExt;
use hula_types::{Battery, RobotState};
use log::{error, info};
use tokio::{
    io::AsyncReadExt,
    net::{UdpSocket, UnixStream},
    select, spawn,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::interval,
};
use tokio_util::sync::CancellationToken;
use zbus::{
    zvariant::{OwnedObjectPath, Value},
    Connection, MatchRule, MessageStream, MessageType, Proxy,
};

const HULA_SOCKET_PATH: &str = "/tmp/hula";
const HULA_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const OS_RELEASE_PATH: &str = "/etc/os-release";

struct HulaService {
    sender: mpsc::Sender<oneshot::Sender<RobotState>>,
}

impl HulaService {
    fn create_sender() -> mpsc::Sender<oneshot::Sender<RobotState>> {
        let (sender, mut receiver) = mpsc::channel::<oneshot::Sender<RobotState>>(1);

        spawn(async move {
            let mut read_buffer = [0; size_of::<RobotState>()];
            let mut stream_option = None;
            let mut interval = interval(HULA_RETRY_TIMEOUT);

            while let Some(response_channel) = receiver.recv().await {
                loop {
                    let mut stream = match stream_option {
                        Some(stream) => stream,
                        None => loop {
                            match UnixStream::connect(HULA_SOCKET_PATH).await {
                                Ok(stream) => break stream,
                                Err(_) => {
                                    info!(
                                        "Could not connect to HuLA socket, trying again in {}s...",
                                        HULA_RETRY_TIMEOUT.as_secs()
                                    );
                                    interval.tick().await;
                                }
                            }
                        },
                    };
                    if stream.read_exact(&mut read_buffer).await.is_ok() {
                        let state = unsafe { read(read_buffer.as_ptr() as *const RobotState) };
                        stream_option = Some(stream);
                        let _ = response_channel.send(state);
                        break;
                    }
                    stream_option = None;
                }
            }
        });

        sender
    }

    pub fn new() -> Self {
        let sender = Self::create_sender();
        Self { sender }
    }

    pub async fn read_from_hula(&mut self) -> Result<RobotState> {
        let (sender, receiver) = oneshot::channel();
        self.sender.send(sender).await?;
        Ok(receiver.await?)
    }
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
        let hulks_os_version = get_hulks_os_version()
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

async fn listen_for_network_change(interface_name: String) -> Result<()> {
    let dbus_connection = Connection::system().await?;

    let link_object = get_link_object(&interface_name, &dbus_connection).await?;

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.network1")?
        .path(link_object)?
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .build();

    let mut stream = MessageStream::for_match_rule(rule, &dbus_connection, Some(1)).await?;

    let mut service_option = None;

    if let Some(ip) = get_ip(&interface_name, &dbus_connection).await? {
        service_option =
            Some(join_multicast(ip, dbus_connection.clone(), interface_name.clone()).await?);
    }

    while let Some(Ok(message)) = stream.next().await {
        if let Ok((_, data, _)) = message.body::<(String, HashMap<String, Value>, Vec<String>)>() {
            if let Some(Value::Str(data)) = data.get("IPv4AddressState") {
                match data.as_str() {
                    "routable" => {
                        info!("IPv4 on {} back online", interface_name);
                        let ip = get_ip(&interface_name, &dbus_connection)
                            .await?
                            .ok_or(eyre!("failed to get IP"))?;
                        service_option = Some(
                            join_multicast(ip, dbus_connection.clone(), interface_name.clone())
                                .await?,
                        );
                    }
                    "off" => {
                        info!("IPv4 on {} offline", interface_name);
                        if let Some(service) = service_option {
                            service.cancel();
                            service.join().await;
                            service_option = None;
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    bail!("failed to get next message")
}

async fn get_link_object(
    interface_name: &str,
    dbus_connection: &Connection,
) -> Result<OwnedObjectPath> {
    let proxy = Proxy::new(
        dbus_connection,
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
            "no DBus path for interface {} found",
            interface_name
        ))
}

async fn get_ip(interface_name: &str, dbus_connection: &Connection) -> Result<Option<Ipv4Addr>> {
    let link_object = get_link_object(interface_name, dbus_connection).await?;

    let proxy = Proxy::new(
        dbus_connection,
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

async fn get_hulks_os_version() -> Result<String> {
    let mut os_release = Ini::new();
    os_release.load_async(OS_RELEASE_PATH).await.unwrap();
    os_release
        .get("default", "VERSION_ID")
        .ok_or_else(|| eyre!("no VERSION_ID in {OS_RELEASE_PATH}"))
}

async fn join_multicast(
    ip: Ipv4Addr,
    dbus_connection: Connection,
    interface_name: String,
) -> Result<AlivenessService> {
    let mut robot_info = RobotInfo::initialize().await?;

    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
        .await
        .wrap_err("failed to bind beacon socket")?;
    socket
        .join_multicast_v4(BEACON_MULTICAST_GROUP, ip)
        .wrap_err_with(|| format!("failed to join multicast group on {}", ip))?;

    info!("Joined multicast on {}", ip);

    let token = CancellationToken::new();

    let handle = {
        let token = token.clone();

        spawn(async move {
            let mut buffer = [0; 1024];
            let mut hula_service = HulaService::new();

            loop {
                select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    result = hula_service.read_from_hula() => {
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
                            &dbus_connection,
                            &interface_name,
                            &robot_info,
                            &buffer[0..num_bytes],
                            peer,
                        )
                        .await.unwrap_or_else(|err| {
                            error!("{err}");
                        });
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

    let interface_name = args().nth(1).unwrap_or_else(|| "enp4s0".to_owned());

    listen_for_network_change(interface_name).await
}

async fn handle_beacon(
    socket: &UdpSocket,
    dbus_connection: &Connection,
    interface_name: &str,
    robot_info: &RobotInfo,
    message: &[u8],
    peer: SocketAddr,
) -> Result<()> {
    if message != BEACON_HEADER {
        bail!("invalid beacon header {message:?}");
    }
    info!("Received beacon from {peer}");
    let system_services = SystemServices::query(dbus_connection).await?;
    let response = AlivenessState {
        hostname: robot_info.hostname.to_owned(),
        interface_name: interface_name.to_owned(),
        system_services,
        hulks_os_version: robot_info.hulks_os_version.to_owned(),
        body_id: robot_info.body_id.to_owned(),
        head_id: robot_info.head_id.to_owned(),
        battery: robot_info.battery.to_owned(),
    };
    let send_buffer = serde_json::to_vec(&response).wrap_err("failed to serialize response")?;
    socket
        .send_to(&send_buffer, peer)
        .await
        .wrap_err_with(|| format!("failed to send beacon response to peer at {peer}"))?;
    Ok(())
}
