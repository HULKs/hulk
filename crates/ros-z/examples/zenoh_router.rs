//! Native Zenoh router for ros-z examples.
//!
//! Run `cargo run -p ros-z --example zenoh_router` when examples should connect
//! through a router instead of peer discovery.
//!
//! Enable logging with `RUST_LOG=info`.

use ros_z::config::RouterConfigBuilder;
use zenoh::Wait;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    zenoh::init_log_from_env_or("error");

    let config = RouterConfigBuilder::new()
        .with_listen_port(7447)
        .build_config()?;

    println!("ROS-Z native router listening on tcp/[::]:7447");
    println!("Press Ctrl-C to stop");

    let session = zenoh::open(config).wait()?;
    println!("Router started with ZID: {}", session.zid());

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(tokio::signal::ctrl_c())?;

    println!("Router shutting down...");
    drop(session);

    Ok(())
}
