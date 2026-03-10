mod completion_edit;

pub mod app;
pub mod config;
pub mod discovery_types;
pub mod protocol;
pub mod timeline_canvas;
pub mod worker;

use color_eyre::{eyre::WrapErr as _, Result};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

pub use app::{ViewerApp, ViewerStartupOverrides};

pub fn setup_logging() -> Result<()> {
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
