//! Lifecycle bringup orchestrator example.
//!
//! Demonstrates using `LifecycleClient` to drive a remote lifecycle node
//! through its full state machine — the pattern used by system bringup
//! managers to coordinate startup and shutdown of multiple nodes.
//!
//! This example spawns a lifecycle node in a background thread, then uses
//! a `LifecycleClient` on the main thread to configure, activate,
//! deactivate, and shut it down.
//!
//! Run with:
//! ```bash
//! cargo run --example lifecycle_bringup
//! ```

// ANCHOR: full_example
use std::time::Duration;

use ros_z::{
    Result,
    context::ContextBuilder,
    lifecycle::{CallbackReturn, LifecycleClient},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Shared Zenoh context — both the lifecycle node and the client connect
    // through the same context (in production they'd typically be separate
    // processes connected via a Zenoh router).
    let context = ContextBuilder::default().build().await?;

    // --- Lifecycle node (simulates a remote node) ---
    let node = context
        .create_lifecycle_node("camera_driver")
        .build()
        .await?;

    node.set_on_configure(|_| {
        println!("[camera_driver] on_configure: opening device");
        CallbackReturn::Success
    });
    node.set_on_activate(|_| {
        println!("[camera_driver] on_activate: streaming");
        CallbackReturn::Success
    });
    node.set_on_deactivate(|_| {
        println!("[camera_driver] on_deactivate: paused");
        CallbackReturn::Success
    });
    node.set_on_shutdown(|_| {
        println!("[camera_driver] on_shutdown: releasing device");
        CallbackReturn::Success
    });

    // Give services a moment to register with Zenoh
    std::thread::sleep(Duration::from_millis(200));

    // --- Bringup manager ---
    let mgr = context.create_node("bringup_manager").build().await?;
    let client = LifecycleClient::new(&mgr, "/camera_driver").await?;

    // Allow time for service discovery
    std::thread::sleep(Duration::from_millis(300));

    let timeout = Duration::from_secs(5);
    {
        // Query the initial state
        let state = client.get_state(timeout).await?;
        println!("[bringup] camera_driver is {:?}", state);

        // Drive the node through its lifecycle
        println!("[bringup] configuring...");
        assert!(client.configure(timeout).await?);

        println!("[bringup] activating...");
        assert!(client.activate(timeout).await?);

        let state = client.get_state(timeout).await?;
        println!("[bringup] camera_driver is now {:?}", state);

        // Simulate some work
        println!("[bringup] node is running... (would do work here)");

        // Graceful shutdown
        println!("[bringup] deactivating...");
        assert!(client.deactivate(timeout).await?);

        println!("[bringup] shutting down...");
        assert!(client.shutdown(timeout).await?);

        let state = client.get_state(timeout).await?;
        println!("[bringup] camera_driver is now {:?}", state);

        Ok(())
    }
}
// ANCHOR_END: full_example
