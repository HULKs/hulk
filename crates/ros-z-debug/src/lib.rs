//! Read-only subscription helpers for `ros-z` debug tooling.
//!
//! `SubscriptionManager` owns debug subscriptions and keeps the latest sample,
//! optional time-window history, status, and update notifications. Relative
//! topic selectors resolve against [`ManagerOptions::target_namespace`], then
//! the manager subscribes to the absolute topic name through `ros-z`.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use ros_z::context::ContextBuilder;
//! use ros_z_debug::{ManagerOptions, RetentionPolicy, SubscriptionManager};
//!
//! # async fn demo() -> ros_z_debug::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = Arc::new(context.create_node("debug").build().await?);
//! let options = ManagerOptions::with_target_namespace("/robot_1")?;
//!
//! let manager = SubscriptionManager::new(node, options);
//! let handle = manager
//!     .subscribe_typed::<String>("status")
//!     .retention(RetentionPolicy::LatestOnly)
//!     .build()
//!     .await?;
//!
//! let latest = handle.latest();
//! # let _ = latest;
//! # Ok(())
//! # }
//! ```

mod error;
mod event;
mod history;
mod manager;
mod retention;
mod sample;
mod status;
mod subscription;
mod topic;

pub use error::{Error, Result};
pub use event::{SubscriptionUpdate, SubscriptionUpdateClosed, SubscriptionUpdateReceiver};
pub use manager::{
    DynamicSubscriptionBuilder, ManagerOptions, SubscriptionManager, TypedSubscriptionBuilder,
};
pub use retention::{DEFAULT_TIME_WINDOW_MAX_SAMPLES, RetentionPolicy, RetentionWindow};
pub use ros_z::dynamic::{
    ByteRenderPolicy, DynamicJsonRenderPolicy as JsonRenderPolicy, NonFiniteFloatRenderPolicy,
    dynamic_payload_to_json, dynamic_value_to_json,
};
pub use sample::{SampleMetadata, SampleRecord};
pub use status::{SubscriptionStatus, SubscriptionStatusSnapshot};
pub use subscription::{JsonSubscriptionHandle, SubscriptionHandle};
pub use topic::{ProjectedTopic, ProjectedTopicScope, TopicProjection, TopicSelector};
