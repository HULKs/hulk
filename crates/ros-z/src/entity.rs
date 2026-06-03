//! Entity types for native ros-z graph records.
//!
//! This module re-exports entity types from ros-z-protocol and adds
//! ros-z-specific extensions.

// Re-export all entity types from ros-z-protocol
pub use ros_z_protocol::entity::*;

use crate::attachment::EndpointGlobalId;

// Constants for ros-z-specific functionality
pub const ADMIN_SPACE: &str = ros_z_protocol::format::ADMIN_SPACE;

pub type Topic = String;

// Extension functions for EndpointEntity

/// Get the global identifier for this endpoint.
pub fn endpoint_global_id(entity: &EndpointEntity) -> EndpointGlobalId {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(entity.node.z_id.to_le_bytes());
    hasher.update(entity.id.to_le_bytes());
    let hash = hasher.finalize();
    let mut endpoint_global_id = [0u8; 16];
    endpoint_global_id.copy_from_slice(&hash[..16]);
    endpoint_global_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use zenoh::session::ZenohId;

    fn node_entity() -> NodeEntity {
        NodeEntity {
            z_id: ZenohId::default(),
            id: 1,
            name: "node".to_string(),
            namespace: "/".to_string(),
        }
    }

    fn endpoint_entity(node: NodeEntity, id: usize) -> EndpointEntity {
        EndpointEntity {
            id,
            node,
            kind: EndpointKind::Publisher,
            topic: "/topic".to_string(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[test]
    fn endpoint_global_id_is_stable_for_same_node_zenoh_id_and_endpoint_local_id() {
        let node = node_entity();
        let endpoint = endpoint_entity(node.clone(), 7);
        let matching_endpoint = endpoint_entity(node, 7);

        assert_eq!(
            endpoint_global_id(&endpoint),
            endpoint_global_id(&matching_endpoint)
        );
    }

    #[test]
    fn endpoint_global_id_changes_when_endpoint_local_id_changes() {
        let node = node_entity();
        let endpoint = endpoint_entity(node.clone(), 7);
        let different_endpoint = endpoint_entity(node, 8);

        assert_ne!(
            endpoint_global_id(&endpoint),
            endpoint_global_id(&different_endpoint)
        );
    }
}
