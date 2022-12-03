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

mod message;
mod aliveness;
