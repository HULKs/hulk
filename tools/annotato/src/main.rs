pub mod ai_assistant;
pub mod annotation;
pub mod annotator_app;
pub mod boundingbox;
pub mod classes;
pub mod label_widget;
pub mod paths;
pub mod remotedata;
pub mod rsync;
pub mod theme;
pub mod utils;
pub mod widgets;

use std::path::PathBuf;

use annotator_app::AnnotatorApp;
use clap::{Parser, Subcommand};
use color_eyre::eyre::{bail, Report, Result};
use eframe::{egui::ViewportBuilder, run_native, NativeOptions, Result as EFrameResult};
use remotedata::DataCommand;
use theme::{apply_theme, MOCHA};

#[derive(Parser, Debug)]
#[clap(name = "annotato")]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Data {
        #[command(subcommand)]
        subcommand: DataCommand,
    },
    Label {
        #[arg(short, long, required = true)]
        dataset_name: String,

        #[arg(short, long, default_value = "false")]
        skip_introduction: bool,
    },
    Ui {
        #[arg(short, long, default_value = ".")]
        image_folder: PathBuf,

        #[arg(short, long, default_value = "data.json")]
        annotation_json: PathBuf,

        #[arg(short, long, default_value = "false")]
        skip_introduction: bool,
    },
}

fn start_labelling_ui(
    image_folder: PathBuf,
    annotation_json: PathBuf,
    skip_introduction: bool,
) -> EFrameResult<()> {
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

    match arguments.command {
        Command::Data { subcommand } => remotedata::handle(&subcommand),
        Command::Label {
            dataset_name,
            skip_introduction,
        } => {
            let image_folder = PathBuf::from_iter(["current", &dataset_name, "images"].iter());
            let annotation_json = PathBuf::from_iter(["current", &dataset_name, "data.json"]);
            if !image_folder.exists() {
                bail!("dataset {dataset_name} not present")
            }
            start_labelling_ui(image_folder, annotation_json, skip_introduction)
                .map_err(|err| Report::msg(err.to_string()))
        }
        Command::Ui {
            image_folder,
            annotation_json,
            skip_introduction,
        } => start_labelling_ui(image_folder, annotation_json, skip_introduction)
            .map_err(|err| Report::msg(err.to_string())),
    }?;

    Ok(())
}
