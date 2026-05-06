//! # ros-z - Zenoh-native robotics middleware in pure Rust
//!
//! `ros-z` provides typed pub/sub and services built directly on
//! [Zenoh](https://zenoh.io), with no C/C++ dependencies.
//!
//! ## Getting started
//!
//! ```rust,ignore
//! use ros_z::prelude::*;
//!
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("talker").build().await?;
//! let publisher = node.publisher::<String>("/chatter").build().await?;
//! publisher.publish(&"hello".to_owned()).await?;
//! ```
//!
//! ## Sync and async APIs
//!
//! Runtime-resource builders (context, node, pub/sub, services, and
//! caches) are async and must be `.build().await`ed inside a Tokio (or
//! compatible) runtime. Local-only builders, such as configuration, schema,
//! dynamic message, and SHM provider builders, remain synchronous.
//!
//! For example, [`Publisher::publish`](pubsub::Publisher::publish) yields to the
//! async executor while Zenoh sends the message, and
//! [`Subscriber::recv`](pubsub::Subscriber::recv) waits asynchronously for the
//! next typed message.
//!
//! ## Imports
//!
//! The easiest way to import all common types is via the prelude:
//!
//! ```rust,ignore
//! use ros_z::prelude::*;
//! ```
//!
//! Or import types individually from their modules.

extern crate self as ros_z;

/// Attachment helpers for carrying metadata alongside messages.
pub mod attachment;
/// Timestamp-indexed, capacity-bounded message cache.
pub mod cache;
/// Configuration types and builder helpers.
pub mod config;
/// Zenoh session context and context builder.
pub mod context;
/// Dynamic (schema-less) message support.
pub mod dynamic;
pub mod encoding;
/// Entity identity types (`SchemaHash`, `TypeInfo`).
pub mod entity;
/// Graph events emitted by the Zenoh network graph.
pub mod event;
/// Native graph introspection (node/topic/service discovery).
pub mod graph;
/// Typed message wrappers and helpers.
pub mod msg;
#[cfg(feature = "nalgebra")]
mod nalgebra_field_type_info;
/// Node creation and management.
pub mod node;
/// Node-local parameter subsystem.
pub mod parameter;
/// Convenience re-exports for common ros-z types.
pub mod prelude;
/// Publishers and subscribers.
pub mod pubsub;
/// Quality-of-Service profiles and options.
pub mod qos;
/// Internal message queues.
pub mod queue;
/// Service client and server.
pub mod service;
/// Shared-memory transport helpers.
pub mod shm;
/// Time and clock primitives for runtime and replay integration.
pub mod time;
/// Topic name validation and manipulation.
pub mod topic_name;
/// Runtime type metadata helpers.
pub mod type_info;
/// Owned Zenoh buffer type.
pub mod zbuf;

#[macro_use]
pub mod utils;

pub use attachment::EndpointGlobalId;
pub use entity::{SchemaHash, TypeInfo};
pub use msg::{EncodedMessage, GeneratedCdrCodec, Message, MessageCodec, SerdeCdrCodec, Service};
pub use ros_z_derive::Message;
pub use type_info::ServiceTypeInfo;
pub use zbuf::ZBuf;
pub use zenoh::Result;

#[doc(hidden)]
pub mod __private {
    pub use ros_z_schema;
    pub use sha2;
}
