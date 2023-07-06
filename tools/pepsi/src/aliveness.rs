use std::{collections::BTreeMap, net::IpAddr, num::ParseIntError, time::Duration};

use clap::{arg, Args};
use color_eyre::owo_colors::OwoColorize;

use crate::parsers::NaoAddress;
use aliveness::{
    query_aliveness,
    service_manager::{ServiceState, SystemServices},
    AlivenessError, AlivenessState,
};
use constants::OS_VERSION;

#[derive(Args)]
pub struct Arguments {
    /// Output verbose version of the aliveness information
    #[arg(long, short = 'v')]
    verbose: bool,
    /// Output aliveness information as json
    #[arg(long, short = 'j')]
    json: bool,
    /// Timeout in ms for waiting for responses
    #[arg(long, short = 't', value_parser = parse_duration, default_value = "200")]
    timeout: Duration,
    /// The NAOs to show the aliveness information from, e.g. 20w or 10.1.24.22
    naos: Option<Vec<NaoAddress>>,
}

fn parse_duration(arg: &str) -> Result<Duration, ParseIntError> {
    let milliseconds = arg.parse()?;
    Ok(Duration::from_millis(milliseconds))
}

type AlivenessList = BTreeMap<IpAddr, AlivenessState>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to query aliveness")]
    QueryFailed(AlivenessError),
    #[error("failed to serialize data")]
    SerializeFailed(serde_json::Error),
}

pub async fn aliveness(arguments: Arguments) -> Result<(), Error> {
    let states = query_aliveness_list(&arguments)
        .await
        .map_err(Error::QueryFailed)?;
    if arguments.json {
        println!(
            "{}",
            serde_json::to_string(&states).map_err(Error::SerializeFailed)?
        );
    } else if arguments.verbose {
        print_verbose(&states);
    } else {
        print_summary(&states);
    }
    Ok(())
}

fn print_summary(states: &AlivenessList) {
    const SPACING: usize = 3;
    const CHARGE_OK_THRESHOLD: f32 = 0.95;
    const CHARGING_ICON: &str = "󱐋";
    const DISCHARGING_ICON: &str = "󰁽";
    const OS_ICON: &str = "󱑞";
    const ALL_OK_ICON: &str = "✔";
    const UNKNOWN_CHARGE_ICON: &str = "󰁽?";
    const NETWORK_ICON: &str = "󰖩 ";
    const TEMPERATURE_ICON: &str = "";

    for (ip, state) in states.iter() {
        let id = match ip {
            IpAddr::V4(ip) => ip.octets()[3],
            IpAddr::V6(ip) => ip.octets()[15],
        };

        let mut output = String::new();

        if let Some(battery) = state.battery {
            if battery.charge < CHARGE_OK_THRESHOLD || battery.current.is_sign_negative() {
                let charge = (battery.charge * 100.0) as u32;
                let icon = if battery.current.is_sign_positive() {
                    CHARGING_ICON
                } else {
                    DISCHARGING_ICON
                };
                output.push_str(&format!("{icon} {charge}%{:SPACING$}", ""))
            }
        } else {
            output.push_str(&format!("{UNKNOWN_CHARGE_ICON}{:SPACING$}", ""))
        }

        let version = &state.hulks_os_version;
        if version != OS_VERSION {
            output.push_str(&format!("{OS_ICON} {version}{:SPACING$}", ""))
        }

        let SystemServices {
            hal,
            hula,
            hulk,
            lola,
        } = state.system_services;
        match hal {
            ServiceState::Active => (),
            _ => output.push_str(&format!("HAL: {hal}{:SPACING$}", "")),
        }
        match hula {
            ServiceState::Active => (),
            _ => output.push_str(&format!("HuLA: {hula}{:SPACING$}", "")),
        }
        match hulk {
            ServiceState::Active => (),
            _ => output.push_str(&format!("HULK: {hulk}{:SPACING$}", "")),
        }
        match lola {
            ServiceState::Active => (),
            _ => output.push_str(&format!("LoLA: {lola}{:SPACING$}", "")),
        }

        if output.is_empty() {
            output = format!("{}{:SPACING$}", ALL_OK_ICON.green().to_string(), "");
        }

        let no_network = "None".to_owned();
        let network = state.network.as_ref().unwrap_or(&no_network);
        let temperature = state
            .temperature
            .map(|temperatures| {
                format!(
                    "{}°C",
                    temperatures.into_lola().into_iter().fold(0.0, f32::max)
                )
            })
            .unwrap_or("?".to_owned());

        println!(
            "[{id}] {output}{NETWORK_ICON} {network}{:SPACING$}{TEMPERATURE_ICON} {temperature}",
            ""
        );
    }
}

fn print_verbose(states: &AlivenessList) {
    const INDENTATION: usize = 2;
    const SPACING: usize = 3;

    for (ip, state) in states.iter() {
        let AlivenessState {
            hostname,
            interface_name,
            system_services,
            hulks_os_version,
            body_id,
            head_id,
            battery,
            network,
            temperature,
        } = state;

        let SystemServices {
            hal,
            hula,
            hulk,
            lola,
        } = system_services;

        let unknown = "Unknown".to_owned();
        let body_id = body_id.as_ref().unwrap_or(&unknown);
        let head_id = head_id.as_ref().unwrap_or(&unknown);
        let battery = battery.map_or_else(
            || unknown.clone(),
            |b| {
                let charge = (b.charge * 100.0) as u32;
                let current = (b.current * 1000.0) as u32;
                format!("Charge: {charge:.0}%{:SPACING$}Current: {current:.0}mA", "")
            },
        );

        let temperature = match temperature {
            Some(temperatures) => {
                let mut temperatures: Vec<_> = temperatures.into_lola().into_iter().collect();
                temperatures.sort_unstable_by(f32::total_cmp);

                let minimum_temperature = temperatures
                    .iter()
                    .next()
                    .expect("temperature array should not be empty");

                let maximum_temperature = temperatures
                    .iter()
                    .last()
                    .expect("temperature array should not be empty");

                let median_temperature = temperatures
                    .iter()
                    .nth(temperatures.len() / 2)
                    .expect("temperature array should not be empty");

                format!(
                    "{}°C / {}°C / {}°C  (minimum / maximum / median)",
                    minimum_temperature, maximum_temperature, median_temperature
                )
            }
            None => unknown.clone(),
        };

        let no_network = "None".to_owned();
        let network = network.as_ref().unwrap_or(&no_network);

        println!(
            "[{ip}]\n\
            {:INDENTATION$}Hostname:          {hostname}\n\
            {:INDENTATION$}Interface name:    {interface_name}\n\
            {:INDENTATION$}HULKs-OS version:  {hulks_os_version}\n\
            {:INDENTATION$}Services:          HAL: {hal}{:SPACING$}HuLA: {hula}{:SPACING$}\
                                              HULK: {hulk}{:SPACING$}LoLA: {lola}\n\
            {:INDENTATION$}Battery:           {battery}\n\
            {:INDENTATION$}Network:           {network}\n\
            {:INDENTATION$}Temperature:       {temperature}\n\
            {:INDENTATION$}Head ID:           {head_id}\n\
            {:INDENTATION$}Body ID:           {body_id}\n",
            "", "", "", "", "", "", "", "", "", "", "", ""
        )
    }
}

async fn query_aliveness_list(arguments: &Arguments) -> Result<AlivenessList, AlivenessError> {
    let ips = arguments
        .naos
        .as_ref()
        .map(|naos| naos.iter().map(|nao| nao.ip).collect());
    let responses = query_aliveness(arguments.timeout, ips).await?;
    Ok(responses.into_iter().collect())
}

pub async fn completions() -> Result<Vec<u8>, AlivenessError> {
    const COMPLETIONS_TIMEOUT: Duration = Duration::from_millis(200);
    let aliveness_states = query_aliveness(COMPLETIONS_TIMEOUT, None).await?;
    let completions = aliveness_states
        .iter()
        .filter_map(|(ip, _)| match ip {
            IpAddr::V4(ip) => Some(ip.octets()[3]),
            _ => None,
        })
        .collect();
    Ok(completions)
}
