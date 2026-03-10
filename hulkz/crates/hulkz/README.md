# Hulkz

**Hulkz** is a high-performance, native Zenoh robotics middleware designed as a ROS 2 replacement. It provides a structured, hierarchical key space for data, configuration, and command flows, optimizing for both machine efficiency (CDR encoding) and human debuggability (JSON).

## Features

- **Native Zenoh**: Built directly on Zenoh for zero-copy, peer-to-peer communication
- **Dual-plane publishing**: CDR for production data, JSON for debugging (lazy serialization)
- **Hierarchical key space**: Five functional planes prevent collision between data, config, and commands
- **Scoped paths**: Intuitive prefix syntax for global (`/`), local, and private (`~/`) data
- **Graph discovery**: Liveliness-based node, session, and publisher discovery
- **Runtime agnostic**: Returns futures, doesn't spawn internally

## Architecture

Hulkz divides the Zenoh key space into five distinct functional planes:

| Plane     | Root Key        | Encoding | Purpose                                    |
|-----------|-----------------|----------|--------------------------------------------|
| **Data**  | `hulkz/data/`   | CDR      | High-bandwidth, low-latency production data |
| **View**  | `hulkz/view/`   | JSON     | Human-readable debug mirror of Data plane  |
| **Param** | `hulkz/param/`  | JSON     | Configuration (read/write branches)        |
| **Graph** | `hulkz/graph/`  | Liveliness | Node/session/publisher discovery         |
| **Cmd**   | `hulkz/cmd/`    | JSON     | RPC services (planned)                     |

### Scoped Path Syntax

Paths use a prefix to express their scope:

| Prefix | Scope   | Example          | Resolves To                                        |
|--------|---------|------------------|----------------------------------------------------|
| `/`    | Global  | `/fleet_status`  | `hulkz/data/global/fleet_status`                   |
| (none) | Local   | `camera/front`   | `hulkz/data/local/{namespace}/camera/front`        |
| `~/`   | Private | `~/debug/state`  | `hulkz/data/private/{namespace}/{node}/debug/state`|

## Quick Start

```rust
use hulkz::{Session, Result};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Odometry {
    x: f64,
    y: f64,
    theta: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a session with a robot namespace
    let session = Session::create("chappie").await?;

    // Create a node (automatically registers in the graph plane)
    let node = session.create_node("navigation").build().await?;

    // Create a publisher (publishes to data + view planes)
    let publisher = node.advertise::<Odometry>("odometry")
        .build()
        .await?;

    // Publish data (explicit timestamp required)
    let timestamp = session.now();
    publisher.put(&Odometry { x: 1.0, y: 2.0, theta: 0.5 }, &timestamp).await?;

    Ok(())
}
```

## Core Types

### Session

Entry point for all hulkz operations. A session connects to the Zenoh network with a specific namespace.

```rust
let session = Session::create("robot_name").await?;

// Discovery APIs
let nodes = session.graph().nodes().list().await?;
let publishers = session.graph().publishers().list().await?;

// Watch for changes (returns watcher + driver future)
let (mut watcher, driver) = session.graph().nodes().watch().await?;
tokio::spawn(driver);  // Must spawn to receive events
while let Some(event) = watcher.recv().await {
    println!("{:?}", event);
}
```

### Node

Nodes are the primary unit of computation. Each node registers itself in the graph plane for discovery.

```rust
let node = session.create_node("perception").build().await?;
```

### Publisher

Publishers send data to a topic on both the data plane (CDR) and view plane (JSON). View plane serialization is lazy - it only happens if there are subscribers.

```rust
let publisher = node.advertise::<MyMessage>("topic_name")
    .build()
    .await?;

// Publish with explicit timestamp (required)
publisher.put(&my_message, &session.now()).await?;

// For derived data, use source timestamp to maintain temporal coherence
publisher.put(&filtered_data, &source_msg.timestamp).await?;
```

### Subscriber

Subscribers receive data from the data plane with configurable buffer depth.

```rust
let mut subscriber = node.subscribe::<MyMessage>("topic_name")
    .build()
    .await?;

loop {
    let msg = subscriber.recv_async().await?;
    println!("Received: {:?}", msg.payload);
}
```

### Parameter

Parameters provide configurable values exposed via the param plane. They support validation and can be loaded from configuration files.

```rust
let (param, driver) = node.declare_parameter::<f64>("~/max_speed")
    .default(1.5)
    .validate(|v| *v > 0.0 && *v <= 10.0)
    .build()
    .await?;
tokio::spawn(driver);

let value = param.get().await;  // Returns Arc<f64>
```

### Buffer

Temporal storage for message lookup by timestamp. Essential for sensor fusion where data from multiple sources needs to be aligned.

```rust
// Create a buffered subscription (200 message capacity)
let (imu_buffer, driver) = node.buffer::<Imu>("imu/data", 200).await?;
tokio::spawn(driver);  // Driver populates buffer

// Look up data at a specific timestamp
let msg = imu_buffer.lookup_nearest(&camera_timestamp).await;
```

## Graph Discovery

Hulkz provides liveliness-based discovery for sessions, nodes, and publishers:

```rust
// List current entities
let sessions = session.graph().sessions().list().await?;
let nodes = session.graph().nodes().list().await?;
let publishers = session.graph().publishers().list().await?;

// Watch for changes (returns watcher + driver future)
let (mut watcher, driver) = session.graph().nodes().watch().await?;
tokio::spawn(driver);

while let Some(event) = watcher.recv().await {
    match event {
        GraphEvent::Joined(info) => println!("Node joined: {}", info.name),
        GraphEvent::Left(info) => println!("Node left: {}", info.name),
    }
}
```

## CLI Tool

See [`hulkz-cli`](../../tools/hulkz-cli/) for introspection and debugging:

```bash
hulkz list nodes              # List active nodes
hulkz list publishers         # List all publishers
hulkz view camera/image       # Subscribe to topic (JSON)
hulkz param get max_speed     # Query parameter
hulkz graph                   # Show network topology
```

## Examples

The `examples/` directory contains runnable examples:

```bash
# Basic pub/sub
cargo run --example publisher    # Publishes odometry at 10 Hz
cargo run --example subscriber   # Receives and prints odometry

# Sensor fusion with temporal alignment
cargo run --example sensor_fusion  # Aligns camera, IMU, and odometry by timestamp

# Filter node with timestamp propagation  
cargo run --example imu_filter     # Subscribe → filter → publish pattern

# Parameters and discovery
cargo run --example parameters   # Parameter declaration and remote config
cargo run --example discovery    # List/watch nodes, publishers, sessions
```

## Logging

`hulkz` emits structured logs via `tracing` but does not install a global subscriber.
Enable logs from your binary with `RUST_LOG`, for example:

```bash
RUST_LOG=hulkz=debug cargo run -p hulkz --example publisher
```

## Documentation

- [API Documentation](https://docs.rs/hulkz) - Full API reference (or run `cargo doc -p hulkz --open`)

## License

See the repository root for license information.
