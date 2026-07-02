use std::sync::Arc;

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use eframe::{
    NativeOptions, Renderer,
    egui_wgpu::{WgpuConfiguration, WgpuSetup},
    run_native,
};
use tokio::runtime::Builder as RuntimeBuilder;
use tracing_subscriber::EnvFilter;

use crate::{app::RobotViewerApp, cli::Arguments};

mod app;
mod cli;
mod scene;
mod state;
mod subscriptions;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("zenoh::net::routing::hat::peer::token=off".parse()?),
        )
        .init();

    let arguments = Arguments::parse();
    let runtime = Arc::new(RuntimeBuilder::new_multi_thread().enable_all().build()?);

    run_native(
        "Robot Viewer",
        NativeOptions {
            renderer: Renderer::Wgpu,
            wgpu_options: wgpu_options(),
            ..Default::default()
        },
        Box::new(move |creation_context| {
            Ok(Box::new(RobotViewerApp::new(
                creation_context,
                arguments.clone(),
                runtime.clone(),
            )))
        }),
    )
    .map_err(|error| eyre!("failed to run robot viewer: {error}"))?;

    Ok(())
}

fn wgpu_options() -> WgpuConfiguration {
    let mut options = WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        let previous = setup.device_descriptor.clone();
        setup.device_descriptor = Arc::new(move |adapter| {
            let mut descriptor = previous(adapter);
            descriptor
                .required_limits
                .max_storage_buffers_per_shader_stage = 9;
            descriptor
        });
    }
    options
}
