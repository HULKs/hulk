use std::{collections::HashMap, error::Error};

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use repository::Repository;

#[derive(Args)]
pub struct Arguments {
    /// Intervals between cycle recordings, e.g. Control=1,VisionTop=30 to record every cycle in Control
    /// and one out of every 30 in VisionTop. Set to 0 or don't specify to disable recording for a cycler.
    #[arg(value_delimiter=',', value_parser = parse_key_value::<String, usize>)]
    pub recording_intervals: Vec<(String, usize)>,
}

pub async fn recording(arguments: Arguments, repository: &Repository) -> Result<()> {
    repository
        .configure_recording_intervals(HashMap::from_iter(arguments.recording_intervals))
        .await
        .wrap_err("failed to set recording settings")
}

pub fn parse_key_value<T, U>(string: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let position = string
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", string))?;
    Ok((string[..position].parse()?, string[position + 1..].parse()?))
}
