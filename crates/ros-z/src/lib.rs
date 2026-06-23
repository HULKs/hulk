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
//! ## Endpoint builders
//!
//! ```rust,ignore
//! let publisher = node.publisher::<String>("/chatter").build().await?;
//! let subscriber = node.subscriber::<String>("/chatter").build().await?;
//! let cache = node.subscriber::<String>("/chatter").cache(200).build().await?;
//! let server = node.service_server::<AddTwoInts>("add_two_ints").build().await?;
//! let client = node.service_client::<AddTwoInts>("add_two_ints").build().await?;
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
mod endpoint_builder;
/// Entity identity types (`SchemaHash`, `TypeInfo`).
pub mod entity;
pub mod error;
/// Native graph introspection (node/topic/service discovery).
pub mod graph;
/// Typed message wrappers and helpers.
pub mod message;
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
pub mod schema;
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

pub use attachment::{ENDPOINT_GLOBAL_ID_SIZE, EndpointGlobalId};
pub use entity::{SchemaHash, TypeInfo};
pub use error::{Error, Result};
pub use message::{Message, SerdeCdrCodec, Service};
pub use ros_z_derive::Message;
pub use schema::{
    EnumSchemaBuilder, MessageSchema, SchemaBuilder, StructSchemaBuilder, TupleVariantSchemaBuilder,
};
pub use type_info::ServiceTypeInfo;
pub use zbuf::ZBuf;

#[doc(hidden)]
pub mod __private {
    pub use ros_z_schema;
}
