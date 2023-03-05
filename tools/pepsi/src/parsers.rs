use std::{
    fmt::{self, Display, Formatter},
    net::Ipv4Addr,
    str::FromStr,
};

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Report, Result,
};
use regex::Regex;

use nao::{Network, SystemctlAction};
use spl_network_messages::PlayerNumber;

pub const SYSTEMCTL_ACTION_POSSIBLE_VALUES: &[&str] =
    &["disable", "enable", "restart", "start", "status", "stop"];

pub fn parse_systemctl_action(systemctl_action: &str) -> Result<SystemctlAction> {
    match systemctl_action {
        "disable" => Ok(SystemctlAction::Disable),
        "enable" => Ok(SystemctlAction::Enable),
        "restart" => Ok(SystemctlAction::Restart),
        "start" => Ok(SystemctlAction::Start),
        "status" => Ok(SystemctlAction::Status),
        "stop" => Ok(SystemctlAction::Stop),
        _ => bail!("unexpected systemctl action"),
    }
}

pub const NETWORK_POSSIBLE_VALUES: &[&str] =
    &["None", "SPL_A", "SPL_B", "SPL_C", "SPL_D", "SPL_E", "SPL_F"];

pub fn parse_network(network: &str) -> Result<Network> {
    match network {
        "None" => Ok(Network::None),
        "SPL_A" => Ok(Network::SplA),
        "SPL_B" => Ok(Network::SplB),
        "SPL_C" => Ok(Network::SplC),
        "SPL_D" => Ok(Network::SplD),
        "SPL_E" => Ok(Network::SplE),
        "SPL_F" => Ok(Network::SplF),
        _ => bail!("unexpected network"),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NaoAddress {
    pub ip: Ipv4Addr,
}

impl FromStr for NaoAddress {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        let expression = Regex::new(r"^(\d+)(w?)$").unwrap();
        match expression.captures(input) {
            Some(captures) => {
                let number = captures
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse()
                    .wrap_err("failed to parse NaoAddress")?;
                let connection = if captures.get(2).unwrap().as_str() == "w" {
                    Connection::Wireless
                } else {
                    Connection::Wired
                };
                let ip =
                    number_to_ip(number, connection).wrap_err("cannot parse from NAO number")?;
                Ok(Self { ip })
            }
            None => Ok(Self {
                ip: input.parse().wrap_err("failed to parse NaoAddress")?,
            }),
        }
    }
}

impl Display for NaoAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.ip.fmt(formatter)
    }
}

#[derive(Debug)]
pub enum Connection {
    Wireless,
    Wired,
}

pub fn number_to_ip(nao_number: u8, connection: Connection) -> Result<Ipv4Addr> {
    if nao_number == 0 || nao_number > 254 {
        bail!("NAO number is either the network (0) or broadcast (255) which is not supported");
    }
    let subnet = match connection {
        Connection::Wireless => 0,
        Connection::Wired => 1,
    };
    Ok(Ipv4Addr::new(10, subnet, 24, nao_number))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaoNumber {
    pub number: u8,
}

impl FromStr for NaoNumber {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self {
            number: input.parse().wrap_err("failed to parse NaoNumber")?,
        })
    }
}

impl Display for NaoNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.number.fmt(formatter)
    }
}

impl TryFrom<NaoAddress> for NaoNumber {
    type Error = Report;

    fn try_from(nao_address: NaoAddress) -> Result<Self> {
        if nao_address.ip.octets()[0] != 10
            || (nao_address.ip.octets()[1] != 0 && nao_address.ip.octets()[1] != 1)
            || nao_address.ip.octets()[2] != 24
        {
            bail!("failed to extract NAO number from IP {nao_address}");
        }

        Ok(Self {
            number: nao_address.ip.octets()[3],
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NaoAddressPlayerAssignment {
    pub nao_address: NaoAddress,
    pub player_number: PlayerNumber,
}

impl FromStr for NaoAddressPlayerAssignment {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        let (prefix, player_number) = parse_assignment(input)?;
        Ok(Self {
            nao_address: prefix.parse()?,
            player_number,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NaoNumberPlayerAssignment {
    pub nao_number: NaoNumber,
    pub player_number: PlayerNumber,
}

impl FromStr for NaoNumberPlayerAssignment {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        let (prefix, player_number) = parse_assignment(input)?;
        Ok(Self {
            nao_number: prefix.parse()?,
            player_number,
        })
    }
}

impl Display for NaoNumberPlayerAssignment {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.nao_number.fmt(formatter)
    }
}

fn parse_assignment(input: &str) -> Result<(&str, PlayerNumber)> {
    let (prefix, player_number) = input.rsplit_once(':').ok_or_else(|| eyre!("missing `:`"))?;
    let player_number = match player_number {
        "1" => PlayerNumber::One,
        "2" => PlayerNumber::Two,
        "3" => PlayerNumber::Three,
        "4" => PlayerNumber::Four,
        "5" => PlayerNumber::Five,
        _ => bail!("unexpected player number {player_number}"),
    };
    Ok((prefix, player_number))
}

impl TryFrom<NaoAddressPlayerAssignment> for NaoNumberPlayerAssignment {
    type Error = Report;

    fn try_from(assignment: NaoAddressPlayerAssignment) -> Result<Self> {
        Ok(Self {
            nao_number: assignment
                .nao_address
                .try_into()
                .wrap_err("failed to convert NAO address into NAO number")?,
            player_number: assignment.player_number,
        })
    }
}
