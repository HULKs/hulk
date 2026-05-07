use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use color_eyre::eyre::{Result, eyre};
use ros_z::{context::ContextBuilder, parameter::NodeParametersExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "twix_demo::TwixDemoConfig")]
#[serde(deny_unknown_fields)]
struct TwixDemoConfig {
    enabled: bool,
    linear_x: f64,
    angular_z: f64,
    label: String,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "tcp/127.0.0.1:7447")]
    endpoint: String,

    #[arg(long)]
    config_root: Option<PathBuf>,

    #[arg(long, default_value = "lab-a")]
    location: String,

    #[arg(long, default_value = "robot-01")]
    robot: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let config_root = args.config_root.unwrap_or_else(default_config_root);
    seed_config(&config_root)?;

    let ctx = ContextBuilder::default()
        .with_mode("client")
        .with_connect_endpoints([args.endpoint.as_str()])
        .with_parameter_layers([
            config_root.join("base"),
            config_root.join(format!("location/{}", args.location)),
            config_root.join(format!("robot/{}", args.robot)),
        ])
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let node = ctx
        .create_node("twix_demo_config")
        .with_namespace("motion")
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let config = node.bind_parameter_as::<TwixDemoConfig>("twix_demo")?;

    config.add_validation_hook(|candidate: &TwixDemoConfig| {
        if candidate.label.trim().is_empty() {
            return Err("label must not be empty".to_string());
        }
        Ok(())
    })?;

    println!("node_fqn=/motion/twix_demo_config");
    println!("config_root={}", config_root.display());
    println!("effective_config={:#?}", config.snapshot().typed());

    std::future::pending::<()>().await;
    Ok(())
}

fn default_config_root() -> PathBuf {
    std::env::temp_dir().join("twix_ros_z_demo_config")
}

fn seed_config(root: &Path) -> Result<()> {
    let path = root.join("base/twix_demo.json5");
    fs::create_dir_all(path.parent().expect("base layer parent"))?;

    let contents = r#"{
  enabled: true,
  linear_x: 0.2,
  angular_z: 0.5,
  label: "Twix Demo"
}
"#;
    fs::write(path, contents)?;
    Ok(())
}
