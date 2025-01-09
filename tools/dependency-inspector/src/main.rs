use app::DependencyInspector;
use eframe::{run_native, NativeOptions};
use repository::{get_repository_root, Repository};
use tokio::runtime::Runtime;

mod app;

fn main() -> Result<(), eframe::Error> {
    let runtime = Runtime::new().unwrap();
    let repository = Repository::new(runtime.block_on(get_repository_root()).unwrap());

    run_native(
        "DependencyInspector",
        NativeOptions::default(),
        Box::new(|creation_context| {
            Ok(Box::new(DependencyInspector::new(
                creation_context,
                repository,
            )))
        }),
    )
}
