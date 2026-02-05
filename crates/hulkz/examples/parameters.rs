//! Parameter example
//!
//! This example demonstrates how to declare and use parameters.
//!
//! Run with: `cargo run --example parameters`

use hulkz::{Result, Session};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a session
    let session = Session::create("demo").await?;
    println!("Session created: {}", session.id());

    // Create a node
    let node = session.create_node("controller").build().await?;
    println!("Node created: controller\n");

    // Declare parameters with default values and validation
    // Note: build() returns (Parameter, driver_future) - spawn the driver
    let (max_speed, max_speed_driver) = node
        .declare_parameter::<f64>("max_speed")
        .default(1.0)
        .validate(|v: &f64| *v > 0.0 && *v <= 10.0)
        .build()
        .await?;
    tokio::spawn(max_speed_driver);
    println!(
        "Declared parameter: max_speed = {:?}",
        max_speed.get().await
    );

    let (enabled, enabled_driver) = node
        .declare_parameter::<bool>("enabled")
        .default(true)
        .build()
        .await?;
    tokio::spawn(enabled_driver);
    println!("Declared parameter: enabled = {:?}", enabled.get().await);

    let (robot_name, robot_name_driver) = node
        .declare_parameter::<String>("name")
        .default("demo_robot".to_string())
        .build()
        .await?;
    tokio::spawn(robot_name_driver);
    println!("Declared parameter: name = {:?}", robot_name.get().await);

    println!("\nWaiting for parameter updates... (Ctrl+C to stop)\n");

    // Simulate using parameters in a control loop
    loop {
        let speed = max_speed.get().await;
        let is_enabled = enabled.get().await;
        let name = robot_name.get().await;

        if *is_enabled {
            println!("[{}] Running at max_speed: {:.2}", name, speed);
        } else {
            println!("[{}] Disabled", name);
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
