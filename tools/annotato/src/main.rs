pub mod ai_assistant;
pub mod annotation;
pub mod annotator_app;
pub mod boundingbox;
pub mod classes;
pub mod label_widget;
pub mod leaderboard;
pub mod paths;
pub mod remotedata;
pub mod rsync;
pub mod theme;
pub mod user_toml;
pub mod utils;
pub mod widgets;

use std::{fs, path::PathBuf};

use annotator_app::AnnotatorApp;
use clap::{Parser, Subcommand};
use color_eyre::eyre::{bail, Report, Result};
use eframe::{egui::ViewportBuilder, run_native, NativeOptions};
use remotedata::DataCommand;
use theme::{apply_theme, MOCHA};

use crate::user_toml::CONFIG;

#[derive(Parser, Debug)]
#[clap(name = "annotato")]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Download and upload datasets
    Data {
        #[command(subcommand)]
        subcommand: DataCommand,
    },
    /// Start labelling data, also downloads and uploads datasets automagically
    Label {
        /// The dataset name to be labelled
        #[arg(required = true)]
        dataset_name: String,
        /// Skips the introduction dialog
        #[arg(short, long, default_value = "false")]
        skip_introduction: bool,
        /// Skips downloading and uploading datasets, command will fail if dataset is not present
        #[arg(short, long, default_value = "false")]
        offline: bool,
    },
}

fn start_labelling_ui(
    image_folder: PathBuf,
    annotation_json: PathBuf,
    skip_introduction: bool,
) -> eframe::Result<()> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder {
            title: Some("annotato-rs".to_string()),
            maximized: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    run_native(
        "annotato-rs",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let context = &cc.egui_ctx;
            apply_theme(context, MOCHA);
            Box::new(
                AnnotatorApp::try_new(cc, image_folder, annotation_json, skip_introduction)
                    .expect("failed to start annotato-rs"),
            )
        }),
    )
}

fn main() -> Result<()> {
    let arguments = Args::parse();

    CONFIG
        .set(toml::from_str(&fs::read_to_string("annotato.toml")?)?)
        .expect("once_cell::set failed");

    match arguments.command {
        Command::Data { subcommand } => remotedata::handle(&subcommand),
        Command::Label {
            dataset_name,
            skip_introduction,
            offline,
        } => {
            let image_folder = PathBuf::from_iter(["current", &dataset_name, "images"].iter());
            let annotation_json = PathBuf::from_iter(["current", &dataset_name, "data.json"]);
            if !image_folder.exists() {
                if offline {
                    bail!("dataset {dataset_name} not present, but offline flag was set")
                }
                println!("dataset {dataset_name} not present, downloading...");
                rsync::rsync_to_local("current", &dataset_name)?;
            }
            start_labelling_ui(image_folder, annotation_json, skip_introduction)
                .map_err(|err| Report::msg(err.to_string()))?;

            if !offline {
                println!("Uploading {dataset_name}...");
                rsync::rsync_to_host("current", &dataset_name)?;
            }

            Ok(())
        }
    }?;

    Ok(())
}
