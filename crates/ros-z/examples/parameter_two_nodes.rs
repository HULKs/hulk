use std::{sync::Arc, time::Duration};

use ros_z::context::ContextBuilder;
use ros_z::parameter::NodeParametersSnapshot;
use ros_z::prelude::*;
use ros_z_msgs::geometry_msgs::{Twist, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_parameters::WalkPublisherParameters")]
#[serde(deny_unknown_fields)]
struct WalkPublisherParameters {
    cmd_vel_topic: String,
    publish_hz: f64,
    linear_x: f64,
    angular_z: f64,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_parameters::WalkMonitorParameters")]
#[serde(deny_unknown_fields)]
struct WalkMonitorParameters {
    cmd_vel_topic: String,
    max_linear_x: f64,
    max_angular_z: f64,
    warn_only: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let context = ContextBuilder::default()
        .with_parameter_layers([
            "./parameters/base",
            "./parameters/location/lab-a",
            "./parameters/robot/robot-01",
        ])
        .build()
        .await?;

    let pub_node = context
        .create_node("walk_publisher")
        .with_namespace("motion")
        .build()
        .await?;
    let sub_node = context
        .create_node("walk_monitor")
        .with_namespace("safety")
        .build()
        .await?;

    let pub_cfg = pub_node.bind_parameter_as::<WalkPublisherParameters>("walk_publisher")?;
    let sub_cfg = sub_node.bind_parameter_as::<WalkMonitorParameters>("walk_monitor")?;

    pub_cfg.add_validation_hook(|cfg: &WalkPublisherParameters| {
        if cfg.publish_hz <= 0.0 {
            return Err("publish_hz must be > 0".into());
        }
        Ok(())
    })?;

    let topic = pub_cfg.snapshot().typed().cmd_vel_topic.clone();
    let zpub = pub_node.publisher::<Twist>(&topic).build().await?;
    let zsub = sub_node.subscriber::<Twist>(&topic).build().await?;

    let pub_cfg_task = pub_cfg.clone();
    tokio::spawn(async move {
        loop {
            let snapshot: Arc<NodeParametersSnapshot<WalkPublisherParameters>> =
                pub_cfg_task.snapshot();
            let cfg = snapshot.typed();
            if cfg.enabled {
                let message = Twist {
                    linear: Vector3 {
                        x: cfg.linear_x,
                        y: 0.0,
                        z: 0.0,
                    },
                    angular: Vector3 {
                        x: 0.0,
                        y: 0.0,
                        z: cfg.angular_z,
                    },
                };
                let _ = zpub.publish(&message).await;
            }
            tokio::time::sleep(Duration::from_secs_f64(1.0 / cfg.publish_hz)).await;
        }
    });

    let sub_cfg_task = sub_cfg.clone();
    tokio::spawn(async move {
        while let Ok(message) = zsub.recv().await {
            let snapshot: Arc<NodeParametersSnapshot<WalkMonitorParameters>> =
                sub_cfg_task.snapshot();
            let cfg = snapshot.typed();
            let linear_ok = message.linear.x.abs() <= cfg.max_linear_x;
            let angular_ok = message.angular.z.abs() <= cfg.max_angular_z;
            if !linear_ok || !angular_ok {
                eprintln!(
                    "cmd_vel limit violation: linear.x={:.2}, angular.z={:.2}",
                    message.linear.x, message.angular.z
                );
            }
        }
    });

    pub_cfg.set_json(
        "linear_x",
        serde_json::json!(0.25),
        "./parameters/robot/robot-01",
    )?;

    std::future::pending::<()>().await;
    Ok(())
}
