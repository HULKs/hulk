use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre::Context as _};
use eframe::{NativeOptions, Renderer, run_native};

use crate::app::LocalizationSimulatorApp;

mod app;
mod cli;
mod scene;

#[derive(Debug, Parser)]
#[command(about = "Deterministic 3D-localization simulator and analysis tool")]
struct CommandLine {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run to completion without a display and emit a complete JSON analysis report.
    Headless(cli::HeadlessArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let command = CommandLine::parse().command;
    if matches!(&command, Some(Command::Headless(_))) {
        tracing_subscriber::fmt()
            .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
            .init();
    } else {
        tracing_subscriber::fmt().init();
    }

    match command {
        Some(Command::Headless(arguments)) => return cli::run(arguments),
        None => {}
    }

    run_native(
        "Localization Simulator",
        NativeOptions {
            renderer: Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(|creation_context| Ok(Box::new(LocalizationSimulatorApp::new(creation_context)))),
    )
    .wrap_err("failed to run localization simulator")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headless_without_requiring_gui_arguments() {
        let command_line = CommandLine::try_parse_from([
            "localization_simulator",
            "headless",
            "--scenario",
            "stationary",
            "--compact",
        ])
        .expect("headless arguments parse");

        assert!(matches!(command_line.command, Some(Command::Headless(_))));
    }

    #[test]
    fn no_subcommand_selects_the_gui() {
        let command_line =
            CommandLine::try_parse_from(["localization_simulator"]).expect("empty arguments parse");

        assert!(command_line.command.is_none());
    }
}
