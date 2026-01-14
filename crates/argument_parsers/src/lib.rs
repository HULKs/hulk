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

use hsl_network_messages::PlayerNumber;
use robot::{Network, SystemctlAction};

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

pub const NETWORK_POSSIBLE_VALUES: &[&str] = &[
    "None",
    "SPL_A",
    "SPL_B",
    "SPL_C",
    "SPL_D",
    "SPL_E",
    "SPL_F",
    "SPL_HULKs",
];

pub fn parse_network(network: &str) -> Result<Network> {
    match network {
        "None" => Ok(Network::None),
        "SPL_A" => Ok(Network::SplA),
        "SPL_B" => Ok(Network::SplB),
        "SPL_C" => Ok(Network::SplC),
        "SPL_D" => Ok(Network::SplD),
        "SPL_E" => Ok(Network::SplE),
        "SPL_F" => Ok(Network::SplF),
        "SPL_HULKs" => Ok(Network::SplHulks),
        _ => bail!("unexpected network"),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct RobotAddress {
    pub ip: Ipv4Addr,
}

impl FromStr for RobotAddress {
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
                    .wrap_err("failed to parse RobotAddress")?;
                let connection = if captures.get(2).unwrap().as_str() == "w" {
                    Connection::Wireless
                } else {
                    Connection::Wired
                };
                let ip =
                    number_to_ip(number, connection).wrap_err("cannot parse from Robot number")?;
                Ok(Self { ip })
            }
            None => Ok(Self {
                ip: input.parse().wrap_err("failed to parse RobotAddress")?,
            }),
        }
    }
}

impl Display for RobotAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.ip.fmt(formatter)
    }
}

#[derive(Debug)]
pub enum Connection {
    Wireless,
    Wired,
}

pub fn number_to_ip(robot_number: u8, connection: Connection) -> Result<Ipv4Addr> {
    if robot_number == 0 || robot_number > 254 {
        bail!("Robot number is either the network (0) or broadcast (255) which is not supported");
    }
    let subnet = match connection {
        Connection::Wireless => 0,
        Connection::Wired => 1,
    };
    Ok(Ipv4Addr::new(10, subnet, 24, robot_number))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RobotNumber {
    pub number: u8,
}

impl FromStr for RobotNumber {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self {
            number: input.parse().wrap_err("failed to parse RobotNumber")?,
        })
    }
}

impl Display for RobotNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.number.fmt(formatter)
    }
}

impl TryFrom<RobotAddress> for RobotNumber {
    type Error = Report;

    fn try_from(robot_address: RobotAddress) -> Result<Self> {
        if robot_address.ip.octets()[0] != 10
            || (robot_address.ip.octets()[1] != 0 && robot_address.ip.octets()[1] != 1)
            || robot_address.ip.octets()[2] != 24
        {
            bail!("failed to extract Robot number from IP {robot_address}");
        }

        Ok(Self {
            number: robot_address.ip.octets()[3],
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RobotAddressPlayerAssignment {
    pub robot_address: RobotAddress,
    pub player_number: PlayerNumber,
}

impl FromStr for RobotAddressPlayerAssignment {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        let (prefix, player_number) = parse_assignment(input)
            .wrap_err_with(|| format!("failed to parse assignment {input}"))?;
        Ok(Self {
            robot_address: prefix
                .parse()
                .wrap_err_with(|| format!("failed to parse robot address {prefix}"))?,
            player_number,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RobotNumberPlayerAssignment {
    pub robot_number: RobotNumber,
    pub player_number: PlayerNumber,
}

impl FromStr for RobotNumberPlayerAssignment {
    type Err = Report;

    fn from_str(input: &str) -> Result<Self> {
        let (prefix, player_number) = parse_assignment(input)?;
        Ok(Self {
            robot_number: prefix.parse()?,
            player_number,
        })
    }
}

impl Display for RobotNumberPlayerAssignment {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.robot_number, self.player_number)
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
        "6" => PlayerNumber::Six,
        "7" => PlayerNumber::Seven,
        _ => bail!("unexpected player number {player_number}"),
    };
    Ok((prefix, player_number))
}

impl TryFrom<RobotAddressPlayerAssignment> for RobotNumberPlayerAssignment {
    type Error = Report;

    fn try_from(assignment: RobotAddressPlayerAssignment) -> Result<Self> {
        Ok(Self {
            robot_number: assignment
                .robot_address
                .try_into()
                .wrap_err("failed to convert Robot address into Robot number")?,
            player_number: assignment.player_number,
        })
    }
}
