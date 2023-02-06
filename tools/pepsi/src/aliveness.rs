use std::{collections::BTreeMap, net::IpAddr, time::Duration};

use aliveness_client::{Aliveness, AlivenessState};
use clap::{arg, Args, Subcommand};
use color_eyre::Result;
use serde::Serialize;

use crate::{
    aliveness_types::{All, Battery, DisplayGrid, Ids, Services, Summary},
    parsers::NaoAddress,
};

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

impl Arguments {
    fn subcommand_arguments(&self) -> &SubcommandArguments {
        match self {
            Arguments::Summary(arguments) => arguments,
            Arguments::Services(arguments) => arguments,
            Arguments::Battery(arguments) => arguments,
            Arguments::Ids(arguments) => arguments,
            Arguments::All(arguments) => arguments,
        }
    }
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

type AlivenessList<T> = BTreeMap<IpAddr, T>;

fn print_grid<T>(data: AlivenessList<T>)
where
    T: DisplayGrid,
{
    const IP_SPACING: usize = 2;
    const COL_SPACING: usize = 3;
    const MAX_COLS: usize = 4;

    let mut col_widths: [usize; MAX_COLS] = [0; MAX_COLS];
    let mut cells = Vec::new();

    for (ip, entry) in data.iter() {
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
        print!("[{}]{:IP_SPACING$}", ip, "");
        for (i, cell) in row.iter().enumerate() {
            let spacing = if i == 0 { 0 } else { COL_SPACING };
            print!("{0:spacing$}{1:<2$}", "", cell, col_widths[i]);
        }
        println!("");
    }
}

pub async fn aliveness(arguments: Arguments) -> Result<()> {
    let subcommand_arguments = arguments.subcommand_arguments();
    let states = query_aliveness(subcommand_arguments).await?;
    let print_json = subcommand_arguments.json;
    match arguments {
        Arguments::Summary(_) => print_states::<Summary>(states, print_json).await?,
        Arguments::Services(_) => print_states::<Services>(states, print_json).await?,
        Arguments::Battery(_) => print_states::<Battery>(states, print_json).await?,
        Arguments::Ids(_) => print_states::<Ids>(states, print_json).await?,
        Arguments::All(_) => print_all(states, print_json).await?,
    };
    Ok(())
}

async fn print_states<T>(states: AlivenessList<AlivenessState>, print_json: bool) -> Result<()>
where
    T: From<AlivenessState> + Serialize + DisplayGrid,
{
    let data: AlivenessList<_> = states
        .into_iter()
        .map(|(ip, state)| (ip, T::from(state)))
        .collect();
    if print_json {
        println!("{}", serde_json::to_string(&data)?);
    } else {
        print_grid(data);
    }
    Ok(())
}

async fn print_all(states: AlivenessList<AlivenessState>, print_json: bool) -> Result<()> {
    let data: AlivenessList<_> = states
        .into_iter()
        .map(|(ip, state)| (ip, All::from(state)))
        .collect();
    if print_json {
        println!("{}", serde_json::to_string(&data)?);
    } else {
        for (ip, entry) in data {
            print!("[{ip}]\n{entry}\n");
        }
    }
    Ok(())
}

async fn query_aliveness(arguments: &SubcommandArguments) -> Result<AlivenessList<AlivenessState>> {
    let timeout = Duration::from_millis(arguments.timeout);
    let ips = arguments
        .naos
        .as_ref()
        .map(|v| v.iter().map(|n| n.ip).collect());
    let responses = Aliveness::query(timeout, ips).await?;
    Ok(responses.into_iter().collect())
}
