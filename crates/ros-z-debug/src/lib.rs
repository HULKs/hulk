//! Read-only subscription helpers for `ros-z` debug tooling.
//!
//! `CachedSubscriptionFactory` owns debug subscriptions and keeps the latest sample,
//! optional time-window history, status, and live update notifications. Relative
//! topic references resolve against [`CachedSubscriptionOptions::target_namespace`],
//! then the factory subscribes to the absolute topic name through `ros-z`.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use ros_z::context::ContextBuilder;
//! use ros_z_debug::{CachedSubscriptionFactory, CachedSubscriptionOptions, RetentionPolicy};
//!
//! # async fn demo() -> ros_z_debug::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = Arc::new(context.create_node("debug").build().await?);
//! let options = CachedSubscriptionOptions::with_target_namespace("/robot_1")?;
//!
//! let factory = CachedSubscriptionFactory::new(node, options);
//! let cache = factory
//!     .subscribe_typed::<String>("status")?
//!     .retention(RetentionPolicy::LatestOnly)
//!     .build()
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

mod cache;
mod error;
mod event;
mod factory;
mod history;
mod observation;
mod retention;
mod sample;
mod status;
mod topic;

pub use cache::{CachedJsonSubscription, CachedSubscription};
pub use error::{Error, Result};
pub use event::{
    CachedSubscriptionUpdate, CachedSubscriptionUpdateClosed, CachedSubscriptionUpdateReceiver,
};
pub use factory::{
    CachedDynamicSubscriptionBuilder, CachedSubscriptionFactory, CachedSubscriptionOptions,
    CachedTypedSubscriptionBuilder,
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
pub use sample::{JsonSampleRecord, SampleMetadata, SampleRecord};
pub use status::{CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot};
pub use topic::{
    ProjectedTopic, ProjectedTopicScope, TargetIdentity, TopicProjection, TopicReference,
};
