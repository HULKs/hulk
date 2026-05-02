//! Async sensor-fusion stream primitives with deterministic persistence boundaries.
//!
//! # Examples
//!
//! Single stream queue + map setup:
//! ```no_run
//! use std::time::Duration;
//! use ros_z::prelude::*;
//! use ros_z_streams::{
//!     CreateAnnouncingPublisher, CreateFutureMapBuilder, LagPolicy,
//! };
//!
//! # async fn demo() -> zenoh::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("demo").build().await?;
//! let publisher = node.announcing_publisher::<String>("sensors/a").await?;
//! let mut map = node
//!     .create_future_map_builder()
//!     .create_future_subscriber::<String>("sensors/a", LagPolicy::Immediate)
//!     .await?
//!     .build();
//!
//! let _ = publisher.announce(ros_z::time::Time::from_nanos(10)).await?;
//! let _ = map.recv().await?;
//! # Ok(())
//! # }
//! ```
//!
//! Two streams where one uses watermark lag policy:
//! ```no_run
//! use std::time::Duration;
//! use ros_z::prelude::*;
//! use ros_z_streams::{CreateFutureMapBuilder, LagPolicy};
//!
//! # async fn demo() -> zenoh::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("fusion").build().await?;
//! let _map = node
//!     .create_future_map_builder()
//!     .create_future_subscriber::<String>("sensors/imu", LagPolicy::Watermark { max_lag: Duration::from_millis(20) })
//!     .await?
//!     .create_future_subscriber::<String>("sensors/vision", LagPolicy::Immediate)
//!     .await?
//!     .build();
//! # Ok(())
//! # }
//! ```

mod announce;
mod future_map;
mod future_queue;

pub use announce::{AnnouncingPublisher, CreateAnnouncingPublisher};
pub use future_map::{
    CreateFutureMapBuilder, FutureItem, FutureMap, FutureMapBuilder, FutureReceive, FutureResult,
};
pub use future_queue::{
    CreateFutureQueue, FutureQueueSubscriber, LagPolicy, LagWarning, QueueEvent, QueueState,
};
