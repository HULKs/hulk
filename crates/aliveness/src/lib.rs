use std::{
    ffi::OsString,
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
};

use log::{debug, error, warn};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io,
    net::UdpSocket,
    spawn,
    sync::mpsc::{channel, Receiver},
};

const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
const BEACON_PORT: u16 = 4242;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum ActiveState {
    Activating,
    Active,
    Deactivating,
    Failed,
    Inactive,
    NotLoaded,
    Reloading,
    Unknown,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct SystemServices {
    pub hal_state: ActiveState,
    pub hula_state: ActiveState,
    pub hulk_state: ActiveState,
    pub lola_state: ActiveState,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AlivenessMessage {
    pub hostname: OsString,
    pub interface_name: String,
    pub system_services: SystemServices,
    pub hulks_os_version: String,
    pub body_id: String,
    pub head_id: String,
    pub battery_charge: f32,
    pub battery_current: f32,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to bind socket to local address")]
    CannotBind(io::Error),
    #[error("failed join beacon multicast group")]
    GroupJoinFailed(io::Error),
}

pub async fn listen_for_beacons(
    interface: Ipv4Addr,
) -> Result<Receiver<(IpAddr, AlivenessMessage)>, Error> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT))
        .await
        .map_err(Error::CannotBind)?;
    socket
        .join_multicast_v4(BEACON_MULTICAST_GROUP, interface)
        .map_err(Error::GroupJoinFailed)?;
    let (sender, receiver) = channel(10);
    let mut buffer = [0; 8192];
    spawn(async move {
        loop {
            let (num_bytes, peer_address) = match socket.recv_from(&mut buffer).await {
                Ok(ok) => ok,
                Err(err) => {
                    error!("failed to receive data from socket: {err}");
                    break;
                }
            };
            debug!("Received {num_bytes} bytes from {peer_address}");
            let message: AlivenessMessage = match serde_json::from_slice(&buffer[0..num_bytes]) {
                Ok(message) => message,
                Err(err) => {
                    warn!("Failed to deserialize beacon message: {err}");
                    continue;
                }
            };
            debug!("{message:?}");
            if let Err(err) = sender.send((peer_address.ip(), message)).await {
                error!("failed to forward aliveness message to channel: {err}");
                break;
            }
        }
    });
    Ok(receiver)
}
