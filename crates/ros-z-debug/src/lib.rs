//! Read-only subscription helpers for `ros-z` debug tooling.
//!
//! `CachedSubscription` handles keep the latest sample, optional time-window
//! history, status, and live update notifications. Drop the last handle to stop
//! the underlying subscription task.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use ros_z::context::ContextBuilder;
//! use ros_z_debug::{CachedSubscriptionNodeExt, RetentionPolicy};
//!
//! # async fn demo() -> ros_z_debug::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = Arc::new(context.create_node("debug").build().await?);
//! let cache = node
//!     .cached_subscription("status")?
//!     .target_namespace("/robot_1")?
//!     .retention(RetentionPolicy::LatestOnly)
//!     .build_typed::<String>()
//!     .await?;
//!
//! let latest = cache.latest();
//! # let _ = latest;
//! # Ok(())
//! # }
//! ```
//!
//! `TopicObserver` spawns observations that keep running after the observer handle
//! is dropped. Drop the returned observation handle to stop its background task.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use ros_z::context::ContextBuilder;
//! use ros_z_debug::{TopicObserver, TopicObserverOptions};
//!
//! # async fn observe_demo() -> ros_z_debug::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = Arc::new(context.create_node("observer").build().await?);
//! let observer = TopicObserver::new(
//!     node,
//!     TopicObserverOptions::with_namespace("/robot_1")?,
//! );
//! let observation = observer.observe_dynamic("status")?.spawn();
//! let status = observation.status();
//! # let _ = status;
//! # Ok(())
//! # }
//! ```

mod builder;
mod cache;
mod error;
mod event;
mod history;
mod observation;
mod retention;
mod sample;
mod status;
mod topic;

pub use builder::{CachedSubscriptionBuilder, CachedSubscriptionNodeExt};
pub use cache::{CachedJsonSubscription, CachedSubscription};
pub use error::{Error, Result};
pub use event::{
    CachedSubscriptionUpdate, CachedSubscriptionUpdateClosed, CachedSubscriptionUpdateReceiver,
};
pub use observation::{
    DynamicTopicObservation, DynamicTopicObservationBuilder, TopicObservation,
    TopicObservationBlockReason, TopicObservationBuilder, TopicObservationStatus,
    TopicObservationUpdate, TopicObservationUpdateClosed, TopicObservationUpdateReceiver,
    TopicObserver, TopicObserverOptions,
};
pub use retention::{DEFAULT_TIME_WINDOW_MAX_SAMPLES, RetentionPolicy, RetentionWindow};
pub use ros_z::dynamic::{
    ByteRenderPolicy, DynamicJsonRenderPolicy as JsonRenderPolicy, NonFiniteFloatRenderPolicy,
    dynamic_payload_to_json, dynamic_value_to_json,
};
pub use sample::{SampleMetadata, SampleRecord};
pub use status::{CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot};
pub use topic::{
    ProjectedTopic, ProjectedTopicScope, TargetIdentity, TopicProjection, TopicReference,
};
