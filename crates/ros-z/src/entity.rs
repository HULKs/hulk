//! Entity types for native ros-z graph records.
//!
//! This module re-exports entity types from ros-z-protocol and adds
//! ros-z-specific extensions.

// Re-export all entity types from ros-z-protocol
pub use ros_z_protocol::entity::*;

use zenoh::{Result, key_expr::KeyExpr};

// Constants for ros-z-specific functionality
pub const ADMIN_SPACE: &str = ros_z_protocol::format::ADMIN_SPACE;

// Type aliases
type NodeName = String;
type NodeNamespace = String;
pub type NodeKey = (NodeNamespace, NodeName);
pub type Topic = String;

// Extension functions for NodeEntity (can't use impl due to orphan rules)

/// Normalize a node namespace for internal storage.
///
/// The root namespace (`"/"`) is stored as an empty string so local and remote
/// entities use the same key representation.
pub fn normalize_node_namespace(namespace: &str) -> String {
    if namespace == "/" {
        String::new()
    } else {
        namespace.to_owned()
    }
}

/// Get the key for this node (namespace, name)
pub fn node_key(entity: &NodeEntity) -> NodeKey {
    (
        normalize_node_namespace(&entity.namespace),
        entity.name.clone(),
    )
}

/// Get the liveliness token key expression for a node
pub fn node_lv_token_key_expr(entity: &NodeEntity) -> Result<KeyExpr<'static>> {
    let key_expr = node_to_liveliness_key_expr(entity)?;
    Ok(key_expr.0)
}

// Extension functions for EndpointEntity

/// Get the global identifier for this endpoint.
pub fn endpoint_global_id(entity: &EndpointEntity) -> crate::attachment::EndpointGlobalId {
    use sha2::Digest;
    let node = entity
        .node
        .as_ref()
        .expect("endpoint_global_id requires endpoint node identity");
    let mut hasher = sha2::Sha256::new();
    hasher.update(node.z_id.to_le_bytes());
    hasher.update(entity.id.to_le_bytes());
    let hash = hasher.finalize();
    let mut endpoint_global_id = [0u8; 16];
    endpoint_global_id.copy_from_slice(&hash[..16]);
    endpoint_global_id
}

// Helper functions for converting entities to LivelinessKE
// Note: Can't implement TryFrom due to orphan rules (both types are from ros-z-protocol)

/// Convert a NodeEntity to a LivelinessKE using the default format
pub fn node_to_liveliness_key_expr(entity: &NodeEntity) -> Result<LivelinessKE> {
    ros_z_protocol::format::node_liveliness_key_expr(entity)
}

/// Convert an EndpointEntity to a LivelinessKE using the default format
pub fn endpoint_to_liveliness_key_expr(entity: &EndpointEntity) -> Result<LivelinessKE> {
    let Some(node) = entity.node.as_ref() else {
        return Err(zenoh::Error::from(
            "endpoint liveliness requires node identity",
        ));
    };
    ros_z_protocol::format::liveliness_key_expr(entity, &node.z_id)
}

/// Convert an Entity to a LivelinessKE using the default format
pub fn entity_to_liveliness_key_expr(entity: &Entity) -> Result<LivelinessKE> {
    match entity {
        Entity::Node(n) => node_to_liveliness_key_expr(n),
        Entity::Endpoint(e) => endpoint_to_liveliness_key_expr(e),
    }
}

/// Get the kind of entity
pub fn entity_kind(entity: &Entity) -> EntityKind {
    match entity {
        Entity::Node(_) => EntityKind::Node,
        Entity::Endpoint(e) => e.entity_kind(),
    }
}

/// Get the endpoint entity if this is an endpoint
pub fn entity_get_endpoint(entity: &Entity) -> Option<&EndpointEntity> {
    match entity {
        Entity::Node(_) => None,
        Entity::Endpoint(e) => Some(e),
    }
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
            enclave: String::new(),
        }
    }

    fn endpoint_entity(node: NodeEntity, id: usize) -> EndpointEntity {
        EndpointEntity {
            id,
            node: Some(node),
            kind: EndpointKind::Publisher,
            topic: "/topic".to_string(),
            type_info: None,
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
