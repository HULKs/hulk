//! Simple publisher example
//!
//! This example demonstrates how to create a session, node, and publisher
//! to send odometry data at 10 Hz.
//!
//! Run with: `cargo run --example publisher`

use hulkz::{Result, Session};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct Odometry {
    x: f64,
    y: f64,
    theta: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a session with namespace "demo"
    let session = Session::create("demo").await?;
    println!("Session created: {}", session.id());

    // Create a node named "robot"
    let node = session.create_node("robot").build().await?;
    println!("Node created: robot");

    // Create a publisher for odometry data
    let publisher = node.advertise::<Odometry>("odometry").build().await?;
    println!("Publishing to: odometry");
    println!("Press Ctrl+C to stop\n");

    // Simulate robot movement
    let mut theta: f64 = 0.0;

    loop {
        // Update position (simple circular motion)
        theta += 0.1;
        let x = 5.0 * theta.cos();
        let y = 5.0 * theta.sin();

        let odom = Odometry { x, y, theta };
        publisher.put(&odom, &session.now()).await?;
        println!("Published: x={:.2}, y={:.2}, theta={:.2}", x, y, theta);

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
