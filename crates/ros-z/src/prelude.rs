//! Convenience re-exports for common ros-z node code.
//!
//! Import `ros_z::prelude::*` to bring the core node-building traits and types
//! into scope. Specialized APIs such as cache, lifecycle, dynamic messages, and
//! action internals remain available from their modules.
//!
//! # Example
//!
//! ```rust,ignore
//! use ros_z::{Result, prelude::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let context = ContextBuilder::default().build().await?;
//!     let node = context.create_node("my_node").build().await?;
//!     // ...
//!     Ok(())
//! }
//! ```

/// Core runtime types.
pub use crate::context::{Context, ContextBuilder};
pub use crate::node::Node;

/// Parameter extension methods on [`Node`].
pub use crate::parameter::NodeParametersExt;

/// Core pub/sub handles and builders.
pub use crate::pubsub::{Publisher, PublisherBuilder, Subscriber, SubscriberBuilder};

/// Core service handles.
pub use crate::service::{RequestId, ServiceClient, ServiceReply, ServiceRequest, ServiceServer};

/// Standard QoS profile object for publisher and subscriber configuration.
pub use crate::qos::QosProfile;

/// Trait bounds and codecs for custom messages and services.
pub use crate::{GeneratedCdrCodec, Message, MessageCodec, SerdeCdrCodec, Service};

/// Type metadata traits for custom message, service, and action definitions.
pub use crate::{
    SchemaHash, TypeInfo,
    type_info::{ActionTypeInfo, ServiceTypeInfo},
};
