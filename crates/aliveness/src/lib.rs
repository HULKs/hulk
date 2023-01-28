use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use aliveness::{BEACON_HEADER, BEACON_MULTICAST_GROUP, BEACON_PORT};
use color_eyre::eyre::{Result, WrapErr};
use futures_util::{stream::FuturesUnordered, StreamExt};
use tokio::{net::UdpSocket, select, time::sleep};

pub use aliveness::{
    service_manager::{ServiceState, SystemServices},
    AlivenessState, Battery,
};

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
        let futures = FuturesUnordered::new();

        for ip in ips {
            futures.push(socket.send_to(BEACON_HEADER, SocketAddrV4::new(ip, BEACON_PORT)));
        }

        let results: Vec<_> = futures.collect().await;
        let result: Result<Vec<_>, _> = results.into_iter().collect();
        result.wrap_err("failed to send beacon via unicast")?;
        Ok(())
    }

    pub async fn query(
        timeout: Duration,
        ips: Option<Vec<Ipv4Addr>>,
    ) -> Result<Vec<AlivenessState>> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .await
            .wrap_err("failed to bind beacon socket")?;

        if let Some(ips) = ips {
            Aliveness::send_beacons_unicast(&socket, ips).await?;
        } else {
            Aliveness::send_beacon_multicast(&socket).await?;
        }

        let mut receive_buffer = [0; 8192];

        let mut aliveness_states = Vec::new();

        loop {
            select! {
                message = socket.recv_from(&mut receive_buffer) => {
                    let (num_bytes, _) = message.wrap_err("failed to receive beacon response")?;
                    aliveness_states.push(serde_json::from_slice(&receive_buffer[0..num_bytes])
                        .wrap_err("failed to deserialize beacon response")?);
                }
                _ = sleep(timeout) => {
                    break Ok(aliveness_states);
                }
            }
        }
    }
}
