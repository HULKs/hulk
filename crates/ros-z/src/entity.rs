//! Entity types for native ros-z graph records.
//!
//! This module re-exports entity types from ros-z-protocol and adds
//! ros-z-specific extensions.

// Re-export all entity types from ros-z-protocol
pub use ros_z_protocol::entity::*;

// Constants for ros-z-specific functionality
pub const ADMIN_SPACE: &str = ros_z_protocol::format::ADMIN_SPACE;

pub type Topic = String;

/// Get the global identifier for this endpoint.
pub fn endpoint_global_id(entity: &EndpointEntity) -> EndpointGlobalId {
    EndpointGlobalId::from(entity)
}
