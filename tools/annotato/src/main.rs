pub mod ai_assistant;
pub mod annotation;
pub mod annotator_app;
pub mod boundingbox;
pub mod classes;
pub mod label_widget;
pub mod paths;

use std::path::PathBuf;

use annotator_app::AnnotatorApp;
use clap::Parser;
use eframe::{run_native, NativeOptions, Result};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value = ".")]
    image_folder: PathBuf,

    #[arg(short, long, default_value = "data.json")]
    annotation_json: PathBuf,

    #[arg(short, long, default_value = "false")]
    skip_introduction: bool,
}

fn main() -> Result<()> {
    let arguments = Args::parse();
    let native_options = NativeOptions::default();

    run_native(
        "annotato-rs",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(AnnotatorApp::try_new(cc, arguments).expect("failed to start annotato-rs"))
        }),
    )
}
