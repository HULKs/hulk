//! Simple subscriber example
//!
//! This example demonstrates how to subscribe to a topic and receive messages.
//!
//! Run with: `cargo run --example subscriber`
//!
//! To test, run the publisher example in another terminal:
//! `cargo run --example publisher`

use hulkz::{Result, Session};
use serde::{Deserialize, Serialize};

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

    // Create a node named "listener"
    let node = session.create_node("listener").build().await?;
    println!("Node created: listener");

    // Subscribe to odometry data
    let mut subscriber = node.subscribe::<Odometry>("odometry").build().await?;

    println!("Subscribed to: odometry");
    println!("Waiting for messages... (run the publisher example in another terminal)\n");

    // Receive and print messages
    loop {
        let msg = subscriber.recv_async().await?;
        println!(
            "Received: x={:.2}, y={:.2}, theta={:.2} @ {:?}",
            msg.payload.x, msg.payload.y, msg.payload.theta, msg.timestamp
        );
    }
}
