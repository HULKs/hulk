use std::{fmt::Display, net::Ipv4Addr, str::FromStr};

use anyhow::Context;

use crate::util::number_to_ip;

pub mod commands;
pub mod config;
pub mod logging;
pub mod naossh;
pub mod util;

pub type NaoNumber = u8;
pub type NaoName = String;
pub type PlayerNumber = u8;

#[derive(Debug)]
pub enum Connection {
    Wireless,
    Wired,
}

#[derive(Debug, Clone, Copy)]
pub struct NaoAddress {
    pub ip: Ipv4Addr,
}

impl FromStr for NaoAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expression = regex::Regex::new(r"^(\d*)(w?)$").unwrap();
        match expression.captures(s) {
            Some(captures) => {
                let number = captures
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse()
                    .context("Failed to parse NaoAddress")?;
                let connection = if captures.get(2).unwrap().as_str() == "w" {
                    Connection::Wireless
                } else {
                    Connection::Wired
                };
                let ip =
                    number_to_ip(number, connection).context("Cannot parse from nao number")?;
                Ok(NaoAddress { ip })
            }
            None => Ok(s.parse().context("Failed to parse NaoAddress")?),
        }
    }
}

impl Display for NaoAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.ip)
    }
}
