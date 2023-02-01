use std::{
    cmp::max,
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    iter::zip,
    net::IpAddr,
    time::Duration,
};

use aliveness_client::{Aliveness, AlivenessState, ServiceState, SystemServices};
use clap::{arg, Args, Subcommand};
use color_eyre::Result;
use serde::Serialize;

use crate::parsers::NaoAddress;

#[derive(Subcommand)]
pub enum Arguments {
    /// Show a summary of the aliveness information
    Summary(SubcommandArguments),
    /// Show the status of the systemd services
    Services(SubcommandArguments),
    /// Show detailed battery information
    Battery(SubcommandArguments),
    /// Show the body and head IDs
    Ids(SubcommandArguments),
    /// Show all information available via aliveness
    All(SubcommandArguments),
}

#[derive(Args)]
pub struct SubcommandArguments {
    /// Output aliveness information as json
    #[arg(long, short = 'j')]
    json: bool,
    /// Timeout in ms for waiting for responses
    #[arg(long, short = 't', default_value = "200")]
    timeout: u64,
    /// The NAOs to show the aliveness information from, e.g. 20w or 10.1.24.22
    naos: Option<Vec<NaoAddress>>,
}

trait DisplayGrid {
    fn format_grid(&self) -> Vec<String>;
}

#[derive(Serialize)]
struct Summary {
    all_services_active: bool,
    battery_charge: Option<f32>,
    hulks_os_version: String,
}

impl From<AlivenessState> for Summary {
    fn from(state: AlivenessState) -> Self {
        Self {
            all_services_active: matches!(
                state.system_services,
                SystemServices {
                    hal: ServiceState::Active,
                    hula: ServiceState::Active,
                    hulk: ServiceState::Active,
                    lola: ServiceState::Active,
                }
            ),
            battery_charge: state.battery.map(|b| b.charge),
            hulks_os_version: state.hulks_os_version,
        }
    }
}

impl DisplayGrid for Summary {
    fn format_grid(&self) -> Vec<String> {
        let service_msg = if self.all_services_active {
            "All services active"
        } else {
            "Some services inactive"
        };
        let battery_msg = if let Some(charge) = self.battery_charge {
            format!("{:.0}%", charge * 100.0)
        } else {
            "Unknown".to_owned()
        };

        vec![
            format!("{}", service_msg),
            format!("HULKs-OS version: {}", self.hulks_os_version),
            format!("Battery: {}", battery_msg),
        ]
    }
}

#[derive(Serialize)]
struct Services {
    services: SystemServices,
}

impl From<AlivenessState> for Services {
    fn from(state: AlivenessState) -> Self {
        Self::from(state.system_services)
    }
}

impl From<aliveness_client::SystemServices> for Services {
    fn from(system_services: aliveness_client::SystemServices) -> Self {
        Self {
            services: system_services,
        }
    }
}

impl DisplayGrid for Services {
    fn format_grid(&self) -> Vec<String> {
        vec![
            format!("HAL: {}", self.services.hal),
            format!("HuLA: {}", self.services.hula),
            format!("HULK: {}", self.services.hulk),
            format!("LoLA: {}", self.services.lola),
        ]
    }
}

#[derive(Serialize)]
struct Battery {
    battery: Option<aliveness_client::Battery>,
}

impl From<AlivenessState> for Battery {
    fn from(state: AlivenessState) -> Self {
        Self::from(state.battery)
    }
}

impl From<Option<aliveness_client::Battery>> for Battery {
    fn from(battery: Option<aliveness_client::Battery>) -> Self {
        Self { battery }
    }
}

impl DisplayGrid for Battery {
    fn format_grid(&self) -> Vec<String> {
        if let Some(battery) = self.battery {
            vec![
                format!("Charge: {:.0}%", battery.charge * 100.0),
                format!("Current: {:.0}mA", battery.current * 1000.0),
            ]
        } else {
            vec![format!("Unknown")]
        }
    }
}

#[derive(Serialize)]
struct Ids {
    body_id: Option<String>,
    head_id: Option<String>,
}

impl From<AlivenessState> for Ids {
    fn from(state: AlivenessState) -> Self {
        Self {
            head_id: state.head_id,
            body_id: state.body_id,
        }
    }
}

impl DisplayGrid for Ids {
    fn format_grid(&self) -> Vec<String> {
        let unknown = "Unknown".to_owned();
        let body_id = self.body_id.as_ref().unwrap_or(&unknown);
        let head_id = self.head_id.as_ref().unwrap_or(&unknown);

        vec![format!("Head ID: {head_id}"), format!("Body ID: {body_id}")]
    }
}

#[derive(Serialize)]
struct All {
    hostname: String,
    interface_name: String,
    hulks_os_version: String,
    #[serde(flatten)]
    services: Services,
    #[serde(flatten)]
    battery: Battery,
    body_id: Option<String>,
    head_id: Option<String>,
}

impl From<AlivenessState> for All {
    fn from(state: AlivenessState) -> Self {
        Self {
            hostname: state.hostname,
            interface_name: state.interface_name,
            hulks_os_version: state.hulks_os_version,
            services: Services::from(state.system_services),
            battery: Battery::from(state.battery),
            body_id: state.body_id,
            head_id: state.head_id,
        }
    }
}

impl Display for All {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const INDENTATION: usize = 2;
        const COL_SPACING: usize = 3;

        let Self {
            hostname,
            interface_name,
            hulks_os_version,
            services,
            battery,
            body_id,
            head_id,
        } = self;

        let service_grid = services.format_grid();
        let battery_grid = battery.format_grid();

        let col_widths: Vec<_> = zip(&service_grid, &battery_grid)
            .map(|(b, s)| max(b.len(), s.len()))
            .collect();

        let mut services = String::new();
        let mut battery = String::new();

        for (i, s) in service_grid.iter().enumerate() {
            let spacing = if i == 0 { 0 } else { COL_SPACING };
            services.push_str(
                format!("{:spacing$}{:2$}", "", s, col_widths.get(i).unwrap_or(&0)).as_str(),
            )
        }
        for (i, b) in battery_grid.iter().enumerate() {
            let spacing = if i == 0 { 0 } else { COL_SPACING };
            battery.push_str(
                format!("{:spacing$}{:2$}", "", b, col_widths.get(i).unwrap_or(&0)).as_str(),
            )
        }

        let unknown = "Unknown".to_owned();
        let body_id = body_id.as_ref().unwrap_or(&unknown);
        let head_id = head_id.as_ref().unwrap_or(&unknown);

        write!(
            f,
            "{:INDENTATION$}Hostname:          {hostname}\n\
            {:INDENTATION$}Interface name:    {interface_name}\n\
            {:INDENTATION$}HULKs-OS version:  {hulks_os_version}\n\
            {:INDENTATION$}Services:          {services}\n\
            {:INDENTATION$}Battery:           {battery}\n\
            {:INDENTATION$}Head ID:           {head_id}\n\
            {:INDENTATION$}Body ID:           {body_id}\n",
            "", "", "", "", "", "", ""
        )
    }
}

#[derive(Serialize)]
struct AlivenessList<T> {
    entries: BTreeMap<IpAddr, T>,
}

impl<T: From<AlivenessState>> From<Vec<(IpAddr, AlivenessState)>> for AlivenessList<T> {
    fn from(states: Vec<(IpAddr, AlivenessState)>) -> Self {
        Self {
            entries: states.into_iter().map(|(ip, s)| (ip, T::from(s))).collect(),
        }
    }
}

impl<T: DisplayGrid> Display for AlivenessList<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const IP_SPACING: usize = 2;
        const COL_SPACING: usize = 3;
        const MAX_COLS: usize = 4;

        let mut col_widths: [usize; MAX_COLS] = [0; MAX_COLS];
        let mut cells = Vec::new();

        for (ip, entry) in self.entries.iter() {
            cells.push((ip, entry.format_grid()));
        }

        for (_, row) in cells.iter() {
            let widths = row.iter().map(|s| s.len());

            for (i, w) in widths.enumerate() {
                if w > col_widths[i] {
                    col_widths[i] = w;
                }
            }
        }

        for (ip, row) in cells.iter() {
            write!(f, "[{}]{:IP_SPACING$}", ip, "")?;
            for (i, cell) in row.iter().enumerate() {
                let spacing = if i == 0 { 0 } else { COL_SPACING };
                write!(f, "{0:spacing$}{1:<2$}", "", cell, col_widths[i])?
            }
            writeln!(f)?
        }

        Ok(())
    }
}

pub async fn aliveness(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::Summary(arguments) => summary(arguments).await?,
        Arguments::Services(arguments) => services(arguments).await?,
        Arguments::Battery(arguments) => battery(arguments).await?,
        Arguments::Ids(arguments) => ids(arguments).await?,
        Arguments::All(arguments) => all(arguments).await?,
    };
    Ok(())
}

async fn summary(arguments: SubcommandArguments) -> Result<()> {
    let aliveness_states = query_aliveness(&arguments).await?;
    let summary = AlivenessList::<Summary>::from(aliveness_states);
    if arguments.json {
        println!("{}", serde_json::to_string(&summary)?);
    } else {
        print!("{summary}");
    }
    Ok(())
}

async fn services(arguments: SubcommandArguments) -> Result<()> {
    let aliveness_states = query_aliveness(&arguments).await?;
    let services = AlivenessList::<Services>::from(aliveness_states);
    if arguments.json {
        println!("{}", serde_json::to_string(&services)?);
    } else {
        print!("{services}");
    }
    Ok(())
}

async fn battery(arguments: SubcommandArguments) -> Result<()> {
    let aliveness_states = query_aliveness(&arguments).await?;
    let battery = AlivenessList::<Battery>::from(aliveness_states);
    if arguments.json {
        println!("{}", serde_json::to_string(&battery)?);
    } else {
        print!("{battery}");
    }
    Ok(())
}

async fn ids(arguments: SubcommandArguments) -> Result<()> {
    let aliveness_states = query_aliveness(&arguments).await?;
    let ids = AlivenessList::<Ids>::from(aliveness_states);
    if arguments.json {
        println!("{}", serde_json::to_string(&ids)?);
    } else {
        print!("{ids}");
    }
    Ok(())
}

async fn all(arguments: SubcommandArguments) -> Result<()> {
    let aliveness_states = query_aliveness(&arguments).await?;
    let all = AlivenessList::<All>::from(aliveness_states);
    if arguments.json {
        println!("{}", serde_json::to_string(&all)?);
    } else {
        for (ip, entry) in all.entries {
            print!("[{ip}]\n{entry}\n");
        }
    }
    Ok(())
}

async fn query_aliveness(arguments: &SubcommandArguments) -> Result<Vec<(IpAddr, AlivenessState)>> {
    let timeout = Duration::from_millis(arguments.timeout);
    let ips = arguments
        .naos
        .as_ref()
        .map(|v| v.iter().map(|n| n.ip).collect());
    Aliveness::query(timeout, ips).await
}
