use std::{collections::BTreeMap, net::IpAddr, num::ParseIntError, time::Duration};

use clap::{arg, Args};
use color_eyre::owo_colors::{OwoColorize, Style};

use crate::parsers::NaoAddress;
use aliveness::{
    query_aliveness,
    service_manager::{ServiceState, SystemServices},
    AlivenessError, AlivenessState, Battery, JointsArray,
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

struct SummaryElements {
    output: String,
}

const SPACING: usize = 3;
const BATTERY_CHARGE_FULL: f32 = 0.95;
const BATTERY_CHARGE_WARN: f32 = 0.7;
const CHARGING_ICON: &str = "󱐋";
const DISCHARGING_ICON: &str = "󰁽";
const OS_ICON: &str = "󱑞";
const ALL_OK_ICON: &str = "✔";
const UNKNOWN_CHARGE_ICON: &str = "󰁽?";
const NETWORK_ICON: &str = "󰖩 ";
const TEMPERATURE_ICON: &str = "";
const TEMPERATURE_WARN_THRESHOLD: f32 = 45.0;
const TEMPERATURE_ERROR_THRESHOLD: f32 = 80.0;

impl SummaryElements {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn show(self) -> String {
        if self.output.is_empty() {
            format!("{}", ALL_OK_ICON.green())
        } else {
            self.output
        }
    }

    fn append(&mut self, icon: &str, text: &str, style: Style) {
        let spacing = " ".repeat(SPACING);
        let output = style.style(format!("{icon} {text} {spacing}")).to_string();
        self.output.push_str(&output);
    }

    fn append_battery(&mut self, battery: &Option<Battery>) {
        let Some(battery) = battery else {
            self.append(UNKNOWN_CHARGE_ICON, "?%", Style::new());
            return;
        };
        let charge = (battery.charge * 100.0) as u32;
        let is_discharging = battery.current.is_sign_negative();
        if is_discharging {
            let style = if battery.charge < BATTERY_CHARGE_WARN {
                Style::new().red()
            } else {
                Style::new().yellow()
            };
            self.append(DISCHARGING_ICON, &format!("{charge}%"), style);
        } else if battery.charge < BATTERY_CHARGE_FULL {
            self.append(CHARGING_ICON, &format!("{charge}%"), Style::new());
        }
    }

    fn append_temperature(&mut self, temperatures: &Option<JointsArray>) {
        let Some(temperatures) = temperatures else {
            self.append(TEMPERATURE_ICON, "?°C", Style::new().blink());
            return;
        };
        let maximum_temperature = temperatures.into_lola().into_iter().fold(0.0, f32::max);
        if maximum_temperature > TEMPERATURE_ERROR_THRESHOLD {
            self.append(
                TEMPERATURE_ICON,
                &format!("{maximum_temperature}°C"),
                Style::new().red(),
            );
        } else if maximum_temperature > TEMPERATURE_WARN_THRESHOLD {
            self.append(
                TEMPERATURE_ICON,
                &format!("{maximum_temperature}°C"),
                Style::new().yellow(),
            );
        }
    }

    fn append_os_version(&mut self, version: &str) {
        if version != OS_VERSION {
            self.append(OS_ICON, version, Style::new());
        }
    }

    fn append_service(&mut self, service: &str, state: ServiceState) {
        match state {
            ServiceState::Active => (),
            ServiceState::Activating => {
                self.append(service, state.to_string().as_str(), Style::new().yellow())
            }
            _ => self.append(service, state.to_string().as_str(), Style::new().red()),
        }
    }
}

fn print_summary(states: &AlivenessList) {
    for (ip, state) in states.iter() {
        let id = match ip {
            IpAddr::V4(ip) => ip.octets()[3],
            IpAddr::V6(ip) => ip.octets()[15],
        };

        let mut output = SummaryElements::new();

        output.append_battery(&state.battery);
        output.append_temperature(&state.temperature);
        output.append_os_version(&state.hulks_os_version);
        let SystemServices {
            hal,
            hula,
            hulk,
            lola,
        } = state.system_services;
        output.append_service("[HAL]", hal);
        output.append_service("[LoLA]", lola);
        output.append_service("[HuLA]", hula);
        output.append_service("[HULK]", hulk);

        let no_network = "None ".to_owned();
        let network = state.network.as_ref().unwrap_or(&no_network);

        println!(
            "[{id}] {NETWORK_ICON} {network}{:SPACING$} {}",
            "",
            output.show()
        );
    }
}

fn print_verbose(states: &AlivenessList) {
    const INDENTATION: usize = 2;

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
                    .first()
                    .expect("temperature array should not be empty");

                let maximum_temperature = temperatures
                    .last()
                    .expect("temperature array should not be empty");

                let median_temperature = temperatures
                    .get(temperatures.len() / 2)
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
