use std::env::current_dir;

use eframe::{run_native, NativeOptions};
use repository::Repository;

use app::DependencyInspector;

mod app;

fn main() -> Result<(), eframe::Error> {
    let current_directory = current_dir().expect("failed to get current directory");
    let repository =
        Repository::find_root(current_directory).expect("failed to find repository root");

    run_native(
        "Vista",
        NativeOptions::default(),
        Box::new(|creation_context| {
            Ok(Box::new(DependencyInspector::new(
                creation_context,
                repository,
            )))
        }),
    )
}
