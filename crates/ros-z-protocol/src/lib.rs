//! Key expression format handling for ros-z.
//!
//! This crate provides key expression generation for mapping ros-z entities
//! (nodes, topics, services, actions) to Zenoh key expressions.
//!
//! # Formats
//!
//! ros-z uses a native key expression format for nodes, topics, services, and actions.
//!
//! # Example
//!
//! ```rust
//! use ros_z_protocol::{entity::*, format};
//!
//! let zid: zenoh::session::ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
//! let node = NodeEntity::new(zid, 0, "my_node".to_string(), "/".to_string());
//!
//! let entity = EndpointEntity {
//!     id: 1,
//!     node,
//!     kind: EndpointKind::Publisher,
//!     topic: "/chatter".to_string(),
//!     type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
//!     qos: Default::default(),
//! };
//!
//! // Generate topic key expression
//! let topic_ke = format::topic_key_expr(&entity).unwrap();
//! ```

pub mod entity;
pub mod error;
pub mod format;
pub mod qos;

pub use entity::{
    ENDPOINT_GLOBAL_ID_SIZE, EndpointEntity, EndpointGlobalId, EndpointKind, Entity, EntityKind,
    NodeEntity, SchemaHash, TypeInfo,
};
pub use error::{ProtocolError, Result};
