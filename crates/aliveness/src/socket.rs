use tokio::{net::UdpSocket, sync::watch::Receiver};
use color_eyre::eyre::{Result, WrapErr};
use tokio::sync::watch;
use dbus_tokio::connection;
use dbus::{message::MatchRule, arg::ReadAll, Message};
use std::{sync::Arc, net::SocketAddrV4};


pub struct SocketWrapper {
    socket: UdpSocket,
    can_send: Receiver<String>,
    network_listener: Network1EventListener,
}

struct Network1EventListener {
}

impl Network1EventListener {
    pub async fn connect<R: ReadAll, F: FnMut(Message, R) -> bool + Send + 'static>(cb: F) -> Result<Self> {
        let (ressource, conn) = connection::new_system_sync()?;

        let _handle = tokio::spawn(async {
            let err = ressource.await;
            panic!("Lost connection to DBus: {}", err);
        });

        let match_rule = MatchRule::new_signal("org.freedesktop.DBus.Properties", "PropertiesChanged")
            .with_sender("org.freedeskop.network1")
            .with_namespaced_path("org/freedesktop/network1/link");
        
        let _incoming_signal = conn.add_match(match_rule).await?.cb(cb);

        Ok(Network1EventListener{})
    }
}

impl SocketWrapper {
    pub async fn new_bind(addr: SocketAddrV4) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;

        let  (tx, rx) = watch::channel("Start".to_owned());

        let network_listener = Network1EventListener::connect(move |_, source: (String,)| {
            // 
            tx.send("Event".to_owned());
            true
        }).await?;

        Ok(SocketWrapper{socket, can_send: rx, network_listener})
    }

    // async fn set_carrier_state(can_send: Arc<Mutex<bool>, CondVar>) {
    //     todo!();
    // }

    pub async fn send_to(&mut self, data: &[u8], addr: &SocketAddrV4) -> Result<usize> {
        while *self.can_send.borrow() != "Start" {
            self.can_send.changed().await;
        }
        self.socket.send_to(data, addr).await.wrap_err("failed to send beacon")
    }
}
