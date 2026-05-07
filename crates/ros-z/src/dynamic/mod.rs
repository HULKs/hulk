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
//! │   SchemaBundle  │────▶│    FieldDef     │
//! │  (type info)    │     │   (field info)  │
//! └────────┬────────┘     └────────┬────────┘
//!          │                       │
//!          ▼                       ▼
//! ┌─────────────────┐     ┌─────────────────┐
//! │ DynamicStruct  │────▶│  DynamicValue   │
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
//! use std::sync::Arc;
//! use ros_z::dynamic::DynamicStruct;
//! use ros_z_schema::{FieldDef, PrimitiveTypeDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName};
//!
//! let name = TypeName::new("geometry_msgs::Point")?;
//! let schema = Arc::new(SchemaBundle {
//!     root: TypeDef::Named(name.clone()),
//!     definitions: TypeDefinitions::from([(
//!         name,
//!         TypeDefinition::Struct(StructDef {
//!             fields: vec![
//!                 FieldDef::new("x", TypeDef::Primitive(PrimitiveTypeDef::F64)),
//!                 FieldDef::new("y", TypeDef::Primitive(PrimitiveTypeDef::F64)),
//!                 FieldDef::new("z", TypeDef::Primitive(PrimitiveTypeDef::F64)),
//!             ],
//!         }),
//!     )]),
//! });
//!
//! // Create and populate a message
//! let mut message = DynamicStruct::default_for_schema(&schema)?;
//! message.set("x", 1.0f64)?;
//! message.set("y", 2.0f64)?;
//! message.set("z", 3.0f64)?;
//!
//! // Serialize to CDR
//! let bytes = message.to_cdr()?;
//!
//! // Deserialize
//! let decoded = DynamicStruct::from_cdr(&bytes, &schema)?;
//! assert_eq!(decoded.get::<f64>("x")?, 1.0);
//! ```

pub mod codec;
pub(crate) mod discovery;
pub mod error;
pub mod message;
pub mod registry;
pub mod schema;
pub mod schema_query;
pub mod schema_service;
pub mod serialization;
pub mod value;

#[cfg(test)]
mod tests;

// Re-export main types
pub use codec::{DynamicCdrCodec, DynamicPayload};
pub use discovery::DiscoveredTopicSchema;
pub use error::DynamicError;
pub use message::{DynamicStruct, DynamicStructBuilder};
pub use registry::{SchemaRegistry, get_root_schema_with_hash, has_schema, register_root_schema};
pub use schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, Schema, SchemaBundle,
    SchemaError, SequenceLengthDef, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
};
pub use schema_query::{
    root_schema_from_response, schema_from_response, schema_from_response_with_hash,
};
pub use schema_service::{
    GetSchema, GetSchemaRequest, GetSchemaResponse, RegisteredSchema, SchemaService,
};
pub use serialization::SerializationFormat;
pub use value::{
    DynamicNamedValue, DynamicValue, EnumPayloadValue, EnumValue, FromDynamic, IntoDynamic,
};

pub(crate) use discovery::{SchemaDiscovery, discovered_schema_type_info};

use crate::pubsub::{Publisher, PublisherBuilder, Subscriber, SubscriberBuilder};

/// Type alias for a dynamic message publisher.
pub type DynamicPublisher = Publisher<DynamicPayload, DynamicCdrCodec>;

/// Type alias for a dynamic message subscriber.
pub type DynamicSubscriber = Subscriber<DynamicPayload, DynamicCdrCodec>;

/// Type alias for a dynamic message publisher builder.
pub type DynamicPublisherBuilder = PublisherBuilder<DynamicPayload, DynamicCdrCodec>;

/// Type alias for a dynamic message subscriber builder.
pub type DynamicSubscriberBuilder = SubscriberBuilder<DynamicPayload, DynamicCdrCodec>;
