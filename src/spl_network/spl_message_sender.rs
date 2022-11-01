use std::net::{Ipv4Addr, SocketAddr};

use log::warn;
use spl_network_messages::{SplMessage, HULKS_TEAM_NUMBER};
use tokio::net::UdpSocket;

pub async fn spl_message_sender(spl_messages: &UdpSocket, message: SplMessage) {
    let message: Vec<u8> = message.into();
    match spl_messages
        .send_to(
            message.as_slice(),
            SocketAddr::new(
                Ipv4Addr::BROADCAST.into(),
                10000 + (HULKS_TEAM_NUMBER as u16),
            ),
        )
        .await
    {
        Ok(_) => {}
        Err(error) => warn!("Failed to send SPL message: {:?}", error),
    }
}
