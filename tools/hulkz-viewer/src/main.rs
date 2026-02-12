mod app;
mod model;
mod timeline_canvas;
mod worker;

use app::{ViewerApp, ViewerStartupOverrides};
use clap::Parser;
use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

#[derive(Debug, Parser)]
#[command(
    name = "hulkz-viewer",
    version,
    about = "egui viewer for hulkz streams"
)]
struct Cli {
    /// Override initial namespace (default: persisted setting or empty/unset)
    #[arg(long, env = "HULKZ_NAMESPACE")]
    namespace: Option<String>,
    /// Override initial source path expression (e.g. "odometry", "/fleet/topic", "~node/private")
    #[arg(long)]
    source: Option<String>,
    /// Use persistent storage at the provided path instead of session temp storage
    #[arg(long)]
    storage_path: Option<PathBuf>,
}

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
    let cli = Cli::parse();

    color_eyre::config::HookBuilder::new()
        .display_env_section(false)
        .install()
        .wrap_err("failed to install color-eyre hook")?;

    setup_logging().wrap_err("viewer startup failed while configuring logging")?;
    let startup_overrides = ViewerStartupOverrides {
        namespace: cli.namespace,
        source_expression: cli.source,
        storage_path: cli.storage_path,
    };

    eframe::run_native(
        "hulkz-viewer",
        eframe::NativeOptions::default(),
        Box::new(move |creation_context| {
            let app = ViewerApp::new(creation_context, startup_overrides.clone())
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|error| eyre!("failed to run eframe event loop: {error}"))?;

    Ok(())
}
