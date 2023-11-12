pub mod annotation;
pub mod annotator_app;
pub mod boundingbox;
pub mod label_widget;
pub mod yolo;
pub mod classes;
pub mod paths;

use annotator_app::AnnotatorApp;
use eframe::{epaint::Vec2, run_native, NativeOptions, Result};

fn main() -> Result<()> {
    let native_options = NativeOptions {
        initial_window_size: Some(Vec2::new(800.0, 600.0)),
        ..Default::default()
    };

    run_native(
        "Rust Annotator",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(AnnotatorApp::try_new(cc).expect("failed to start annotator"))
        }),
    )
}
