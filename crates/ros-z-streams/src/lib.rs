//! Async sensor-fusion stream primitives with deterministic persistence boundaries.
//!
//! This crate provides tools to seamlessly fuse multiple asynchronous data streams
//! (like IMU, Vision, or Lidar) into a single, perfectly time-ordered pipeline. It handles
//! out-of-order delivery, network jitter, and variable sensor latencies by using
//! ahead-of-time timestamp announcements and wall-clock safety boundaries.
//!
//! # Core Concepts
//! * **Announcing Publishers**: Emit lightweight timestamp markers *before* sending heavy payloads, allowing downstream nodes to anticipate data.
//! * **Safety Lag**: A wall-clock `Duration` representing the maximum expected physical transit delay for a stream. It guarantees data is held in a temporary buffer long enough for delayed announcements to arrive.
//! * **Future Map**: A multi-stream fusion engine that holds data in a `temporary` buffer until it is mathematically safe, then releases it exactly once into a strictly time-ordered `persistent` map.
//!
//! # Examples
//!
//! ## Single Stream Setup
//! A basic queue and map setup with a 10ms safety lag to account for network jitter.
//!
//! ```no_run
//! use std::time::Duration;
//! use ros_z::prelude::*;
//! use ros_z_streams::{
//!     CreateAnnouncingPublisher, CreateFutureMapBuilder,
//! };
//!
//! # async fn demo() -> ros_z::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("demo").build().await?;
//!
//! let publisher = node.announcing_publisher::<String>("sensors/a").await?;
//!
//! let mut map = node
//!     .create_future_map_builder()
//!     // Wait up to 10ms for delayed announcements before finalizing boundaries
//!     .create_future_subscriber::<String>("sensors/a", Duration::from_millis(10))
//!     .await?
//!     .build();
//!
//! let pending = publisher.announce(ros_z::time::Time::from_nanos(10)).await?;
//! pending.publish(&"payload".to_string()).await?;
//!
//! let item = map.recv().await?;
//! println!("Persistent historical data: {:?}", item.persistent);
//! println!("Temporary future data: {:?}", item.temporary);
//! # Ok(())
//! # }
//! ```
//!
//! ## Multi-Stream Sensor Fusion
//! Fusing a high-frequency, low-latency IMU stream with a lower-frequency, high-latency Vision stream.
//!
//! ```no_run
//! use std::time::Duration;
//! use ros_z::prelude::*;
//! use ros_z_streams::CreateFutureMapBuilder;
//!
//! # async fn demo() -> ros_z::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("fusion").build().await?;
//!
//! let mut map = node
//!     .create_future_map_builder()
//!     // IMU packets arrive quickly over hardware buses (5ms lag tolerance)
//!     .create_future_subscriber::<String>("sensors/imu", Duration::from_millis(5))
//!     .await?
//!     // Vision packets take longer to encode and transmit (50ms lag tolerance)
//!     .create_future_subscriber::<String>("sensors/vision", Duration::from_millis(50))
//!     .await?
//!     .build();
//!
//! // As data arrives, `map.recv()` will dynamically calculate the global safe time,
//! // ensuring IMU and Vision frames are perfectly matched by timestamp.
//! # Ok(())
//! # }
//! ```

mod announce;
mod future_map;
mod future_queue;

pub use announce::{AnnouncingPublisher, CreateAnnouncingPublisher};
pub use future_map::{
    CreateFutureMapBuilder, FutureItem, FutureMap, FutureMapBuilder, FutureResult,
};
pub use future_queue::{CreateFutureQueue, FutureQueueSubscriber};
