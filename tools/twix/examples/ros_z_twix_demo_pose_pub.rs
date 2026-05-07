use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::{Result, eyre};
use ros_z::context::ContextBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, ros_z::Message)]
#[message(name = "twix_demo::RobotPose")]
struct RobotPose {
    x: f64,
    y: f64,
    theta: f64,
    confidence: f64,
    state: String,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "tcp/127.0.0.1:7447")]
    endpoint: String,

    #[arg(long, default_value = "/twix_demo/robot_pose")]
    topic: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let ctx = ContextBuilder::default()
        .with_mode("client")
        .with_connect_endpoints([args.endpoint.as_str()])
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let node = ctx
        .create_node("twix_demo_pose_publisher")
        .with_namespace("tools")
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let publisher = node
        .publisher::<RobotPose>(&args.topic)
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;

    let mut tick = 0_u64;
    loop {
        let phase = tick as f64 / 20.0;
        let pose = RobotPose {
            x: phase.cos() * 2.5,
            y: phase.sin() * 1.5,
            theta: phase,
            confidence: 0.6 + ((phase / 2.0).sin() + 1.0) * 0.2,
            state: if tick % 80 < 40 {
                "tracking".to_string()
            } else {
                "recovering".to_string()
            },
        };

        publisher
            .publish(&pose)
            .await
            .map_err(|error| eyre!(error.to_string()))?;
        tick += 1;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
