pub mod annotation;
pub mod annotator_app;
pub mod boundingbox;
pub mod label_widget;
pub mod classes;
pub mod paths;
pub mod ai_assistant;

use annotator_app::AnnotatorApp;
use eframe::{run_native, NativeOptions, Result};

fn main() -> Result<()> {
    let native_options = NativeOptions::default();

    run_native(
        "annotato-rs",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(AnnotatorApp::try_new(cc).expect("failed to start annotato-rs"))
        }),
    )
}
