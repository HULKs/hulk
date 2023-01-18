use std::{
    collections::HashMap,
    env::VarError,
    error::Error,
    mem::size_of,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ptr::read,
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use dbus::{
    arg::Variant,
    message::MatchRule,
    nonblock::{self, Proxy},
    Path
};
use dbus_tokio::connection;
use hula_types::{Battery, RobotState};
use serde::Serialize;
use serde_json::Value;
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

// #[derive(Debug)]
// enum CarrierState {
//     NoCarrier,
//     Off,
//     Degraded,
//     Configuring,
//     Routable,
//     Configured,
// }

// impl TryFrom<&Variant<String>> for CarrierState {
//     type Error = String;

//     fn try_from(value: &Variant<String>) -> std::result::Result<Self, Self::Error> {
//         println!("Match on '{}'", value.0);
//         match &value.0[..] {
//             "off" => Ok(CarrierState::Off),
//             "no-carrier" => Ok(CarrierState::NoCarrier),
//             "degraded" => Ok(CarrierState::Degraded),
//             "configuring" => Ok(CarrierState::Configuring),
//             "routable" => Ok(CarrierState::Routable),
//             "configured" => Ok(CarrierState::Configured),
//             _ => Err(format!("carrier state {} is unknown", value.0)),
//         }
//     }
// }

async fn get_ip() -> Result<Option<Ipv4Addr>> {
    // Connect to the system dbus and retrieve the Ipv4Addr from the network1 object
    let (ressource, conn) = connection::new_system_sync()?;

    let _handle = tokio::spawn(async {
        let err = ressource.await;
        panic!("Lost connection to DBus: {}", err);
    });

    // Get the link objects
    let proxy = Proxy::new(
        "org.freedesktop.network1",
        "/org/freedesktop/network1",
        Duration::from_secs(2),
        conn.clone(),
    );

    let (links,): (Vec<(i32, String, Path<'static>)>,) = proxy
        .method_call("org.freedesktop.network1.Manager", "ListLinks", ())
        .await?;
    println!("Available links: {:?}", links);
    // TODO: Not hardcode to object _32

    for link_path in links.filter_map(|(_, interface_name, link_path)| {
        if ["enp4s0", "wlan0"].contains(interface_name) {
            Some(link_path)
        } else {
            None
        }
    }) {
        
    }

    let proxy = Proxy::new(
        "org.freedesktop.network1",
        "/org/freedesktop/network1/link/_32",
        Duration::from_secs(2),
        conn,
    );

    let (description,): (String,) = proxy
        .method_call("org.freedesktop.network1.Link", "Describe", ())
        .await?;

    let description: Value = serde_json::from_str(&description)?;

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

async fn listen_for_network_change(socket: Arc<UdpSocket>) -> Result<()> {
    // Re-enter the multicast group whenever connection is regained
    let (resource, conn) = connection::new_system_sync()?;

    let handle = tokio::spawn(async {
        resource.await;
    });

    let match_rule = MatchRule::new_signal("org.freedesktop.DBus.Properties", "PropertiesChanged")
        .with_sender("org.freedesktop.network1")
        .with_namespaced_path("/org/freedesktop/network1/link");

    let signal = conn
        .add_match(match_rule)
        .await?
        .cb(|msg, (_,): (String,)| {
            if let (_, Some(data2)) = msg.get2::<String, HashMap<String, Variant<String>>>() {
                match data2.get("IPv4AddressState").map(|variant| &variant.0[..]) {
                    Some("routable") => {
                        // let ip = get_ip().await.expect("No ip").unwrap();
                        // socket
                        //     .join_multicast_v4(BEACON_MULTICAST_GROUP, ip)
                        //     .wrap_err_with(|| format!("failed to join multicast group on {ip}"))
                        //     .expect("Error handling missing");
                        println!("IPv4 Back online")
                    }
                    Some("off") => println!("IPv4 offline"),
                    _ => (),
                }
            }

            true
        });

    handle.await?;

    conn.remove_match(signal.token()).await?;
    bail!("Lost connection to DBus")
}

async fn join_multicast(socket: Arc<UdpSocket>) -> Result<()> {
    if let Some(ip) = get_ip().await? {
        println!("Joining multicast {ip}");
        socket
            .join_multicast_v4(BEACON_MULTICAST_GROUP, ip)
            .wrap_err_with(|| format!("failed to join multicast group on {ip}"))?;
    }
    tokio::spawn(async move {
        if let Err(error) = listen_for_network_change(socket).await {
            println!("{error}");
        }
    });
    Ok(())
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
    let socket = Arc::new(
        UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
            .await
            .wrap_err("failed to bind beacon socket")?,
    );
    // socket
    //     .join_multicast_v4(BEACON_MULTICAST_GROUP, interface_ip)
    //     .wrap_err_with(|| format!("failed to join multicast group on {interface_ip}"))?;
    join_multicast(socket.clone()).await?;
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
