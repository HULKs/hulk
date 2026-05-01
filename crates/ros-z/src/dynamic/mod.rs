//! Dynamic message support for ros-z.
//!
//! This module provides runtime message handling where message types are
//! determined at runtime rather than compile time. This is useful for:
//!
//! - Generic tools that work with any message type (rosbag, echo, etc.)
//! - Dynamic tooling for DDS/CDR payloads
//! - Dynamic message inspection and modification
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │  MessageSchema  │────▶│   FieldSchema   │
//! │  (type info)    │     │   (field info)  │
//! └────────┬────────┘     └────────┬────────┘
//!          │                       │
//!          ▼                       ▼
//! ┌─────────────────┐     ┌─────────────────┐
//! │ DynamicMessage  │────▶│  DynamicValue   │
//! │   (container)   │     │    (values)     │
//! └────────┬────────┘     └─────────────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  CDR Serialize  │
//! │  /Deserialize   │
//! └─────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use ros_z::dynamic::{MessageSchema, DynamicMessage, FieldType};
//!
//! // Create a schema for geometry_msgs::Point
//! let schema = MessageSchema::builder("geometry_msgs::Point")
//!     .field("x", FieldType::Float64)
//!     .field("y", FieldType::Float64)
//!     .field("z", FieldType::Float64)
//!     .build()?;
//!
//! // Create and populate a message
//! let mut message = DynamicMessage::new(&schema);
//! message.set("x", 1.0f64)?;
//! message.set("y", 2.0f64)?;
//! message.set("z", 3.0f64)?;
//!
//! // Serialize to CDR
//! let bytes = message.to_cdr()?;
//!
//! // Deserialize
//! let decoded = DynamicMessage::from_cdr(&bytes, &schema)?;
//! assert_eq!(decoded.get::<f64>("x")?, 1.0);
//! ```

pub mod codec;
pub(crate) mod discovery;
pub mod error;
pub mod message;
pub mod registry;
pub mod schema;
pub mod schema_bridge;
pub mod schema_query;
pub mod schema_service;
pub mod serialization;
pub(crate) mod type_info;
pub mod value;

#[cfg(test)]
mod tests;

// Re-export main types
pub use codec::DynamicCdrCodec;
pub use discovery::DiscoveredTopicSchema;
pub use error::DynamicError;
pub use message::{DynamicMessage, DynamicMessageBuilder};
pub use registry::{SchemaRegistry, get_schema, get_schema_with_hash, has_schema, register_schema};
pub use schema::{
    EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldSchema, FieldType, MessageSchema,
};
pub use schema_bridge::{bundle_to_message_schema, message_schema_to_bundle};
pub use schema_query::{schema_from_response, schema_from_response_with_hash};
pub use schema_service::{
    GetSchema, GetSchemaRequest, GetSchemaResponse, RegisteredSchema, SchemaService,
};
pub use serialization::SerializationFormat;
pub use type_info::schema_hash;
pub use value::{
    DynamicNamedValue, DynamicValue, EnumPayloadValue, EnumValue, FromDynamic, IntoDynamic,
};

pub(crate) use discovery::{SchemaDiscovery, discovered_schema_type_info};
pub(crate) use type_info::schema_type_info;

use crate::pubsub::{Publisher, PublisherBuilder, Subscriber, SubscriberBuilder};

/// Type alias for a dynamic message publisher.
pub type DynamicPublisher = Publisher<DynamicMessage, DynamicCdrCodec>;

/// Type alias for a dynamic message subscriber.
pub type DynamicSubscriber = Subscriber<DynamicMessage, DynamicCdrCodec>;

/// Type alias for a dynamic message publisher builder.
pub type DynamicPublisherBuilder = PublisherBuilder<DynamicMessage, DynamicCdrCodec>;

/// Type alias for a dynamic message subscriber builder.
pub type DynamicSubscriberBuilder = SubscriberBuilder<DynamicMessage, DynamicCdrCodec>;
