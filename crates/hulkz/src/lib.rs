//! # hulkz
//!
//! A native Zenoh robotics middleware designed as a ROS 2 replacement.
//!
//! hulkz provides a structured key space for data, configuration, and commands, with dual-encoding
//! (CDR for performance, JSON for debugging) and automatic network discovery.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use hulkz::{Session, Result};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct Odometry { x: f64, y: f64, theta: f64 }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let session = Session::create("robot").await?;
//!     let node = session.create_node("nav").build().await?;
//!
//!     // Publish
//!     let publisher = node.advertise::<Odometry>("odom").build().await?;
//!     publisher.put(&Odometry { x: 1.0, y: 2.0, theta: 0.5 }, &session.now()).await?;
//!
//!     // Subscribe
//!     let mut subscriber = node.subscribe::<Odometry>("odom").build().await?;
//!     let msg = subscriber.recv_async().await?;
//!     println!("Received: {:?}", msg.payload);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Core Types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Session`] | Connection to Zenoh network with namespace context |
//! | [`Node`] | Unit of computation, registered for discovery |
//! | [`Publisher`] | Sends data to data plane (CDR) + view plane (JSON) |
//! | [`Subscriber`] | Receives data from a topic |
//! | [`Parameter`] | Runtime-configurable values with remote read/write |
//!
//! # Functional Planes
//!
//! hulkz divides the Zenoh key space into five planes:
//!
//! | Plane | Key Prefix | Encoding | Purpose |
//! |-------|------------|----------|---------|
//! | **Data** | `hulkz/data/` | CDR | Production data streams |
//! | **View** | `hulkz/view/` | JSON | Debug mirror |
//! | **Param** | `hulkz/param/` | JSON | Configuration (read/write branches) |
//! | **Graph** | `hulkz/graph/` | Liveliness | Node/publisher discovery |
//! | **Cmd** | `hulkz/cmd/` | JSON | RPC services (planned) |
//!
//! # Scoped Paths
//!
//! Topics use prefix syntax to define their visibility scope:
//!
//! | Prefix | Scope | Example | Expands To |
//! |--------|-------|---------|------------|
//! | `/` | Global | `/fleet_status` | `hulkz/data/global/fleet_status` |
//! | (none) | Local | `camera/front` | `hulkz/data/local/{ns}/camera/front` |
//! | `~/` | Private | `~/debug` | `hulkz/data/private/{ns}/{node}/debug` |
//!
//! ```rust
//! use hulkz::ScopedPath;
//!
//! let global: ScopedPath = "/fleet_status".try_into().unwrap();
//! let local: ScopedPath = "camera/front".try_into().unwrap();
//! let private: ScopedPath = "~/debug".try_into().unwrap();
//! ```
//!
//! # Timestamps
//!
//! [`Publisher::put`] requires an explicit timestamp. This forces consideration of temporal
//! semantics:
//!
//! - **Sensor data**: Use [`Session::now()`] - the capture time
//! - **Derived data**: Use the source message's timestamp for temporal coherence
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize, Deserialize)] struct Reading { value: f64 }
//! # #[derive(Serialize, Deserialize)] struct Filtered { value: f64 }
//! # #[derive(Serialize, Deserialize)] struct Imu { accel: f64 }
//! # struct Filter; impl Filter { fn update(&self, _: &Imu) -> Filtered { Filtered { value: 0.0 } } }
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! # let node = session.create_node("n").build().await?;
//! # let publisher = node.advertise::<Reading>("sensor").build().await?;
//! # let filter_pub = node.advertise::<Filtered>("filtered").build().await?;
//! # let reading = Reading { value: 1.0 };
//! # let filter = Filter;
//! # let mut imu_sub = node.subscribe::<Imu>("imu").build().await?;
//! # let imu_msg = imu_sub.recv_async().await?;
//! // Sensor: current time
//! publisher.put(&reading, &session.now()).await?;
//!
//! // Filter: inherit source timestamp
//! let filtered = filter.update(&imu_msg.payload);
//! filter_pub.put(&filtered, &imu_msg.timestamp).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Temporal Alignment
//!
//! Use [`Buffer`] for sensor fusion where data must be aligned by timestamp:
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Clone, Serialize, Deserialize)] struct Imu { accel: f64 }
//! # #[derive(Clone, Serialize, Deserialize)] struct Camera { frame: u32 }
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! # let node = session.create_node("fusion").build().await?;
//! # let mut camera_sub = node.subscribe::<Camera>("camera").build().await?;
//! // Create buffered subscriptions
//! let (imu, driver) = node.buffer::<Imu>("imu", 200).await?;
//! tokio::spawn(driver);
//!
//! // Look up IMU reading at camera's timestamp
//! let camera_msg = camera_sub.recv_async().await?;
//! let imu_msg = imu.lookup_nearest(&camera_msg.timestamp).await;
//! # Ok(())
//! # }
//! ```
//!
//! # Discovery
//!
//! Sessions, nodes, and publishers register via liveliness tokens for automatic discovery:
//!
//! ```rust,no_run
//! # use hulkz::{Session, NodeEvent, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! // List current state
//! let nodes = session.list_nodes().await?;
//! let publishers = session.list_publishers().await?;
//!
//! // Watch for changes
//! let (mut watcher, driver) = session.watch_nodes().await?;
//! tokio::spawn(driver);
//! while let Some(event) = watcher.recv().await {
//!     match event {
//!         NodeEvent::Joined(name) => println!("+ {name}"),
//!         NodeEvent::Left(name) => println!("- {name}"),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Runtime Agnostic
//!
//! hulkz returns futures and does not spawn tasks internally. Callers control task spawning and
//! error handling. APIs that require background processing return `(Handle, Driver)` tuples -
//! spawn the driver on your runtime.

#[doc(inline)]
pub use crate::{
    buffer::{Buffer, BufferBuilder},
    cache::Cache,
    config::Config,
    error::{Error, Result, ScopedPathError},
    graph::{
        EntityAccess, GraphAccess, GraphEvent, NodeInfo, ParameterInfo, PublisherInfo, SessionInfo,
        Watcher,
    },
    message::Message,
    node::Node,
    parameter::Parameter,
    publisher::Publisher,
    scoped_path::{Scope, ScopedPath},
    session::{ParamAccessBuilder, Session},
    subscriber::Subscriber,
};

pub mod buffer;
pub mod cache;
pub mod config;
pub mod error;
pub mod graph;
pub mod message;
pub mod node;
pub mod parameter;
pub mod publisher;
pub mod scoped_path;
pub mod session;
pub mod subscriber;

mod key;

/// Zenoh timestamp - used for temporal ordering and alignment.
///
/// Obtain via [`Session::now()`] or from received [`Message::timestamp`].
pub type Timestamp = zenoh::time::Timestamp;
