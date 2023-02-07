use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use color_eyre::eyre::{Result, WrapErr};
use futures_util::{stream::FuturesUnordered, StreamExt};
use tokio::{net::UdpSocket, select, time::sleep};

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
}

pub struct Aliveness {}

impl Aliveness {
    async fn send_beacon_multicast(socket: &UdpSocket) -> Result<()> {
        socket
            .send_to(
                BEACON_HEADER,
                SocketAddrV4::new(BEACON_MULTICAST_GROUP, BEACON_PORT),
            )
            .await
            .wrap_err("failed to send beacon via multicast")?;
        Ok(())
    }

    async fn send_beacons_unicast(socket: &UdpSocket, ips: Vec<Ipv4Addr>) -> Result<()> {
        let futures: FuturesUnordered<_> = ips
            .into_iter()
            .map(|ip| socket.send_to(BEACON_HEADER, SocketAddrV4::new(ip, BEACON_PORT)))
            .collect();

        let results: Vec<_> = futures.collect().await;
        let result: Result<Vec<_>, _> = results.into_iter().collect();
        result.wrap_err("failed to send beacon via unicast")?;
        Ok(())
    }

    pub async fn query(
        timeout: Duration,
        ips: Option<Vec<Ipv4Addr>>,
    ) -> Result<Vec<(IpAddr, AlivenessState)>> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .await
            .wrap_err("failed to bind beacon socket")?;

        if let Some(ips) = ips {
            Self::send_beacons_unicast(&socket, ips).await?;
        } else {
            Self::send_beacon_multicast(&socket).await?;
        }

        let mut receive_buffer = [0; 8192];

        let mut aliveness_states = Vec::new();

        loop {
            select! {
                message = socket.recv_from(&mut receive_buffer) => {
                    let (num_bytes, peer) = message.wrap_err("failed to receive beacon response")?;
                    aliveness_states.push((peer.ip(), serde_json::from_slice(&receive_buffer[0..num_bytes])
                        .wrap_err("failed to deserialize beacon response")?));
                }
                _ = sleep(timeout) => {
                    break Ok(aliveness_states);
                }
            }
        }
    }
}
