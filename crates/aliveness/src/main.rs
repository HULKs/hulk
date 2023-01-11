use std::net::{Ipv4Addr, SocketAddrV4};

use color_eyre::eyre::{Result, WrapErr};
use hula_types::Battery;
use serde::Deserialize;
use tokio::net::UdpSocket;

mod socket;

const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
const BEACON_PORT: u16 = 4242;
const BEACON_HEADER: &[u8; 6] = b"BEACON";

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
    pub hal: ActiveState,
    pub hula: ActiveState,
    pub hulk: ActiveState,
    pub lola: ActiveState,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BeaconResponse {
    pub hostname: String,
    pub interface_name: String,
    pub system_services: SystemServices,
    pub hulks_os_version: String,
    pub body_id: String,
    pub head_id: String,
    pub battery: Battery,
}

pub struct Aliveness {}

impl Aliveness {
    pub async fn serve() -> Result<Self> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(10, 0, 24, 89), 0))
            .await
            .wrap_err("failed to bind beacon socket")?;
        socket
            .send_to(
                BEACON_HEADER,
                SocketAddrV4::new(BEACON_MULTICAST_GROUP, BEACON_PORT),
            )
            .await
            .wrap_err("failed to send beacon")?;


        

        
        loop {
            println!("Waiting for responses");
            let mut receive_buffer = [0; 8192];
            let (num_bytes, peer) = socket
                .recv_from(&mut receive_buffer)
                .await
                .wrap_err("failed to receive beacon response")?;
            println!("From {peer}, {num_bytes} bytes");
            let response: BeaconResponse = serde_json::from_slice(&receive_buffer[0..num_bytes])
                .wrap_err("failed to deserialize beacon response")?;
            println!("{response:#?}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Start");
    Aliveness::serve().await?;
    Ok(())
}
