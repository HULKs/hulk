use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
};

use log::{debug, error, warn};
use thiserror::Error;
use tokio::{net::UdpSocket, spawn};

const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
const BEACON_PORT: u16 = 4242;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to bind socket to local address")]
    CannotBind(io::Error),
    #[error("failed join beacon multicast group")]
    GroupJoinFailed(io::Error),
}

pub struct Aliveness {}

impl Aliveness {
    pub async fn serve() -> Result<Self, Error> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .await
            .map_err(Error::CannotBind)?;
        spawn(async move {
            loop {
            }
        });
        Ok(Self {})
    }
}
