//! Key expression format handling for ros-z.
//!
//! This crate provides key expression generation for mapping ros-z entities
//! (nodes, topics, services, actions) to Zenoh key expressions.
//!
//! # Formats
//!
//! ros-z uses a native key expression format for nodes, topics, services, and actions.
//!
//! # no_std Support
//!
//! This crate is `no_std` compatible with `alloc`:
//!
//! ```toml
//! [dependencies]
//! ros-z-protocol = { version = "0.1", default-features = false }
//! ```
//!
//! # Example
//!
//! ```rust
//! use ros_z_protocol::{entity::*, format};
//!
//! let zid: zenoh::session::ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
//! let node = NodeEntity::new(0, zid, 0, "my_node".to_string(), "/".to_string(), String::new());
//!
//! let entity = EndpointEntity {
//!     id: 1,
//!     node: Some(node),
//!     kind: EndpointKind::Publisher,
//!     topic: "/chatter".to_string(),
//!     type_info: None,
//!     qos: Default::default(),
//! };
//!
//! // Generate topic key expression
//! let topic_ke = format::topic_key_expr(&entity).unwrap();
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod entity;
pub mod format;
pub mod qos;

pub use entity::{
    EndpointEntity, EndpointKind, Entity, EntityKind, NodeEntity, SchemaHash, TypeInfo,
};
