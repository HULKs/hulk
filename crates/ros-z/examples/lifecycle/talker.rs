//! Lifecycle-managed talker node.
//!
//! Demonstrates the ros-z lifecycle state machine:
//! - Registers `on_configure` / `on_activate` / `on_deactivate` callbacks
//! - Creates a lifecycle-gated publisher for `std_msgs::String`
//! - Drives the node through its full lifecycle programmatically
//!
//! Run with:
//! ```bash
//! cargo run --example lifecycle_talker
//! ```

// ANCHOR: full_example
use ros_z::{Result, context::ContextBuilder, lifecycle::CallbackReturn};
use ros_z_msgs::std_msgs::String as RosString;

#[tokio::main]
async fn main() -> Result<()> {
    // Build a Zenoh context and create a lifecycle node.
    // The node starts in the Unconfigured state.
    let context = ContextBuilder::default().build().await?;
    let mut node = context
        .create_lifecycle_node("lifecycle_talker")
        .build()
        .await?;

    // Register callbacks for each lifecycle transition.
    // Each callback receives the previous state and must return
    // CallbackReturn::Success, ::Failure, or ::Error.
    node.set_on_configure(|_prev| {
        // Load parameters, open files, connect to hardware here.
        println!("[configure] loading parameters");
        CallbackReturn::Success
    });

    node.set_on_activate(|_prev| {
        // Start timers, enable hardware outputs here.
        println!("[activate] publisher enabled");
        CallbackReturn::Success
    });

    node.set_on_deactivate(|_prev| {
        // Pause timers, disable hardware outputs here.
        println!("[deactivate] publisher paused");
        CallbackReturn::Success
    });

    node.set_on_cleanup(|_prev| {
        // Release resources acquired in on_configure here.
        println!("[cleanup] releasing resources");
        CallbackReturn::Success
    });

    // Create a lifecycle-gated publisher.
    // It is registered as a managed entity: activate()/deactivate() on the
    // node will automatically gate this publisher. While deactivated,
    // publish() returns Ok(()) but silently drops the message.
    let pub_ = node.create_publisher::<RosString>("chatter").await?;

    // configure(): Unconfigured → Inactive
    // on_configure callback fires; publisher remains deactivated.
    node.configure().await?;

    // This publish is silently dropped — the node is Inactive.
    pub_.publish(&RosString {
        data: "dropped (inactive)".to_string(),
    })
    .await?;

    // activate(): Inactive → Active
    // on_activate callback fires; publisher is now live.
    node.activate().await?;

    // Messages are delivered while the node is Active.
    for i in 0..5 {
        let message = RosString {
            data: format!("hello {i}"),
        };
        pub_.publish(&message).await?;
        println!("published: {}", message.data);
    }

    // deactivate(): Active → Inactive — publisher is gated again.
    node.deactivate().await?;
    // cleanup(): Inactive → Unconfigured — release resources.
    node.cleanup().await?;
    // shutdown(): Unconfigured → Finalized — terminal state.
    node.shutdown().await?;

    println!("node finalized");
    Ok(())
}
// ANCHOR_END: full_example
