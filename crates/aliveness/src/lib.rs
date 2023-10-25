use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use futures_util::{stream::FuturesUnordered, StreamExt};
use hula_types::JointsArray;
use tokio::{net::UdpSocket, time};

pub use hula_types::Battery;
use serde::{Deserialize, Serialize};
use service_manager::SystemServices;

pub mod service_manager;

pub const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
pub const BEACON_PORT: u16 = 4242;
pub const BEACON_HEADER: &[u8; 6] = b"BEACON";

#[derive(Debug, Serialize, Deserialize)]
pub struct AlivenessState {
    pub hostname: String,
    pub interface_name: String,
    pub system_services: SystemServices,
    pub hulks_os_version: String,
    pub body_id: Option<String>,
    pub head_id: Option<String>,
    pub battery: Option<Battery>,
    pub network: Option<String>,
    pub temperature: Option<JointsArray>,
}

#[derive(Debug, thiserror::Error)]
pub enum AlivenessError {
    #[error("failed to send beacon via multicast")]
    MulticastNotSent(io::Error),
    #[error("failed to send beacon via unicast")]
    UnicastNotSent(Vec<io::Error>),
    #[error("failed to bind beacon socket")]
    SocketNotBound(io::Error),
    #[error("failed to receive beacon response")]
    ReceiveFailed(io::Error),
    #[error("failed to deserialize JSON")]
    DeserializeFailed(serde_json::Error),
}

async fn send_beacon_multicast(socket: &UdpSocket) -> Result<(), AlivenessError> {
    socket
        .send_to(
            BEACON_HEADER,
            SocketAddrV4::new(BEACON_MULTICAST_GROUP, BEACON_PORT),
        )
        .await
        .map_err(AlivenessError::MulticastNotSent)?;
    Ok(())
}

async fn send_beacons_unicast(
    socket: &UdpSocket,
    ips: Vec<Ipv4Addr>,
) -> Result<(), AlivenessError> {
    let results: Vec<_> = ips
        .into_iter()
        .map(|ip| socket.send_to(BEACON_HEADER, SocketAddrV4::new(ip, BEACON_PORT)))
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;

    let errors: Vec<_> = results
        .into_iter()
        .filter_map(|result| match result {
            Err(error) => Some(error),
            Ok(_) => None,
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AlivenessError::UnicastNotSent(errors))
    }
}

pub async fn query_aliveness(
    timeout: Duration,
    ips: Option<Vec<Ipv4Addr>>,
) -> Result<Vec<(IpAddr, AlivenessState)>, AlivenessError> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .await
        .map_err(AlivenessError::SocketNotBound)?;

    if let Some(ips) = ips {
        send_beacons_unicast(&socket, ips).await?;
    } else {
        send_beacon_multicast(&socket).await?;
    }

    let mut receive_buffer = [0; 8192];

    let mut aliveness_states = Vec::new();

    loop {
        if let Ok(message) = time::timeout(timeout, socket.recv_from(&mut receive_buffer)).await {
            let (num_bytes, peer) = message.map_err(AlivenessError::ReceiveFailed)?;
            aliveness_states.push((
                peer.ip(),
                serde_json::from_slice(&receive_buffer[0..num_bytes])
                    .map_err(AlivenessError::DeserializeFailed)?,
            ));
        } else {
            break Ok(aliveness_states);
        }
    }
}

pub fn query_aliveness_sync(
    timeout: Duration,
    ips: Option<Vec<Ipv4Addr>>,
) -> Result<Vec<(IpAddr, AlivenessState)>, AlivenessError> {
    let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
    let inner = rt.block_on(query_aliveness(timeout, ips));

    return inner;
}
