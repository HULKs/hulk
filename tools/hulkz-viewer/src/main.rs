mod app;
mod model;
mod worker;

use app::ViewerApp;
use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

fn setup_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hulkz_viewer=info,hulkz_stream=info,warn"));

    let layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_level(true)
        .with_file(false)
        .with_line_number(false)
        .compact();

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
        .wrap_err("failed to initialize tracing subscriber")?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::new()
        .display_env_section(false)
        .install()
        .wrap_err("failed to install color-eyre hook")?;

    setup_logging().wrap_err("viewer startup failed while configuring logging")?;

    eframe::run_native(
        "hulkz-viewer",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            let app = ViewerApp::new(creation_context)
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|error| eyre!("failed to run eframe event loop: {error}"))?;

    Ok(())
}
