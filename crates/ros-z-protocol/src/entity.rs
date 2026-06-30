//! ros-z entity types for key expression generation.

use core::{fmt::Display, ops::Deref};
pub use ros_z_schema::SchemaHash;
use serde::{Deserialize, Serialize};
use std::string::String;
use zenoh::{key_expr::KeyExpr, session::ZenohId};

use crate::qos::{QosDecodeError, QosProfile};

/// Placeholder for empty namespace.
pub const EMPTY_PLACEHOLDER: &str = "%";

/// Liveliness key expression wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LivelinessKE(pub KeyExpr<'static>);

impl LivelinessKE {
    pub fn new(ke: KeyExpr<'static>) -> Self {
        Self(ke)
    }
}

impl Deref for LivelinessKE {
    type Target = KeyExpr<'static>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Topic key expression wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TopicKE(KeyExpr<'static>);

impl TopicKE {
    pub fn new(ke: KeyExpr<'static>) -> Self {
        Self(ke)
    }
}

impl Deref for TopicKE {
    type Target = KeyExpr<'static>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for TopicKE {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Number of bytes in an endpoint global identifier.
pub const ENDPOINT_GLOBAL_ID_SIZE: usize = 16;

/// Stable protocol-visible identifier for a ros-z endpoint.
///
/// Use `EndpointGlobalId::from(&endpoint)` to derive the identifier from an
/// [`EndpointEntity`]. Use [`EndpointGlobalId::as_bytes`] or
/// [`EndpointGlobalId::into_bytes`] when raw wire-layout bytes are needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EndpointGlobalId([u8; ENDPOINT_GLOBAL_ID_SIZE]);

impl EndpointGlobalId {
    /// All-zero endpoint identifier.
    pub const ZERO: Self = Self([0; ENDPOINT_GLOBAL_ID_SIZE]);

    /// Create an endpoint global identifier from its raw bytes.
    pub fn new(bytes: [u8; ENDPOINT_GLOBAL_ID_SIZE]) -> Self {
        Self(bytes)
    }

    /// Return the raw endpoint identifier bytes.
    pub fn as_bytes(&self) -> &[u8; ENDPOINT_GLOBAL_ID_SIZE] {
        &self.0
    }

    /// Return the raw endpoint identifier bytes.
    pub fn into_bytes(self) -> [u8; ENDPOINT_GLOBAL_ID_SIZE] {
        self.0
    }
}

impl Display for EndpointGlobalId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in self.as_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl From<[u8; ENDPOINT_GLOBAL_ID_SIZE]> for EndpointGlobalId {
    fn from(bytes: [u8; ENDPOINT_GLOBAL_ID_SIZE]) -> Self {
        Self::new(bytes)
    }
}

impl From<EndpointGlobalId> for [u8; ENDPOINT_GLOBAL_ID_SIZE] {
    fn from(value: EndpointGlobalId) -> Self {
        value.into_bytes()
    }
}

/// Namespace/name key used to index graph entities by node.
pub type NodeKey = (String, String);

/// Normalize a node namespace for internal graph indexing.
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

/// ros-z node entity.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct NodeEntity {
    pub z_id: ZenohId,
    pub id: usize,
    pub name: String,
    pub namespace: String,
}

impl NodeEntity {
    pub fn new(z_id: ZenohId, id: usize, name: String, namespace: String) -> Self {
        Self {
            z_id,
            id,
            name,
            namespace,
        }
    }

    pub fn fully_qualified_name(&self) -> String {
        fully_qualified_node_name(&self.namespace, &self.name)
    }

    /// Return this node's namespace in the graph-index representation.
    pub fn normalized_namespace(&self) -> String {
        normalize_node_namespace(&self.namespace)
    }

    /// Return the graph index key for this node.
    pub fn key(&self) -> NodeKey {
        (self.normalized_namespace(), self.name.clone())
    }

    /// Convert this node into its native ros-z liveliness key expression.
    ///
    /// This remains fallible because Zenoh validates key-expression syntax and
    /// `NodeEntity` can contain arbitrary names/namespaces.
    pub fn liveliness_key_expr(&self) -> crate::Result<LivelinessKE> {
        crate::format::node_liveliness_key_expr(self)
    }
}

pub fn fully_qualified_node_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() || namespace == "/" {
        format!("/{name}")
    } else {
        format!("/{}/{}", namespace.trim_start_matches('/'), name)
    }
}

/// ros-z entity kind (node, publisher, subscription, service, client).
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Node,
    Publisher,
    Subscription,
    Service,
    Client,
}

impl Display for EntityKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EntityKind::Node => write!(f, "NN"),
            EntityKind::Publisher => write!(f, "MP"),
            EntityKind::Subscription => write!(f, "MS"),
            EntityKind::Service => write!(f, "SS"),
            EntityKind::Client => write!(f, "SC"),
        }
    }
}

impl core::str::FromStr for EntityKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NN" => Ok(EntityKind::Node),
            "MP" => Ok(EntityKind::Publisher),
            "MS" => Ok(EntityKind::Subscription),
            "SS" => Ok(EntityKind::Service),
            "SC" => Ok(EntityKind::Client),
            _ => Err("Invalid entity kind"),
        }
    }
}

/// ros-z endpoint kind (publisher, subscription, service, client).
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum EndpointKind {
    Publisher,
    Subscription,
    Service,
    Client,
}

impl Display for EndpointKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EndpointKind::Publisher => write!(f, "MP"),
            EndpointKind::Subscription => write!(f, "MS"),
            EndpointKind::Service => write!(f, "SS"),
            EndpointKind::Client => write!(f, "SC"),
        }
    }
}

impl core::str::FromStr for EndpointKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MP" => Ok(EndpointKind::Publisher),
            "MS" => Ok(EndpointKind::Subscription),
            "SS" => Ok(EndpointKind::Service),
            "SC" => Ok(EndpointKind::Client),
            _ => Err("Invalid endpoint kind"),
        }
    }
}

impl From<EndpointKind> for EntityKind {
    fn from(kind: EndpointKind) -> Self {
        match kind {
            EndpointKind::Publisher => EntityKind::Publisher,
            EndpointKind::Subscription => EntityKind::Subscription,
            EndpointKind::Service => EntityKind::Service,
            EndpointKind::Client => EntityKind::Client,
        }
    }
}

impl TryFrom<EntityKind> for EndpointKind {
    type Error = &'static str;

    fn try_from(kind: EntityKind) -> Result<Self, Self::Error> {
        match kind {
            EntityKind::Node => Err("Node is not a valid endpoint kind"),
            EntityKind::Publisher => Ok(EndpointKind::Publisher),
            EntityKind::Subscription => Ok(EndpointKind::Subscription),
            EntityKind::Service => Ok(EndpointKind::Service),
            EntityKind::Client => Ok(EndpointKind::Client),
        }
    }
}

/// Type information (name + schema hash).
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub hash: SchemaHash,
}

impl TypeInfo {
    pub fn new(name: impl Into<String>, hash: SchemaHash) -> Self {
        TypeInfo {
            name: name.into(),
            hash,
        }
    }
}

/// ros-z endpoint entity (publisher, subscription, service, client).
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct EndpointEntity {
    pub id: usize,
    pub node: NodeEntity,
    pub kind: EndpointKind,
    pub topic: String,
    pub type_info: TypeInfo,
    pub qos: QosProfile,
}

impl EndpointEntity {
    pub fn entity_kind(&self) -> EntityKind {
        self.kind.into()
    }

    /// Convert this endpoint into its native ros-z liveliness key expression.
    ///
    /// This remains fallible because Zenoh validates key-expression syntax and
    /// endpoint names, topics, and type names are represented as strings.
    pub fn liveliness_key_expr(&self) -> crate::Result<LivelinessKE> {
        crate::format::liveliness_key_expr(self, &self.node.z_id)
    }
}

impl From<&EndpointEntity> for EndpointGlobalId {
    fn from(endpoint: &EndpointEntity) -> Self {
        use sha2::Digest;

        let mut hasher = sha2::Sha256::new();
        hasher.update(endpoint.node.z_id.to_le_bytes());
        hasher.update((endpoint.id as u64).to_le_bytes());
        let hash = hasher.finalize();
        let mut bytes = [0u8; ENDPOINT_GLOBAL_ID_SIZE];
        bytes.copy_from_slice(&hash[..ENDPOINT_GLOBAL_ID_SIZE]);
        Self::new(bytes)
    }
}

/// Generic ros-z entity (node or endpoint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entity {
    Node(NodeEntity),
    Endpoint(EndpointEntity),
}

impl Entity {
    /// Return the semantic kind of this entity.
    pub fn kind(&self) -> EntityKind {
        match self {
            Entity::Node(_) => EntityKind::Node,
            Entity::Endpoint(endpoint) => endpoint.entity_kind(),
        }
    }

    /// Return the endpoint payload when this entity is an endpoint.
    pub fn as_endpoint(&self) -> Option<&EndpointEntity> {
        match self {
            Entity::Node(_) => None,
            Entity::Endpoint(endpoint) => Some(endpoint),
        }
    }

    /// Convert this entity into its native ros-z liveliness key expression.
    pub fn liveliness_key_expr(&self) -> crate::Result<LivelinessKE> {
        match self {
            Entity::Node(node) => node.liveliness_key_expr(),
            Entity::Endpoint(endpoint) => endpoint.liveliness_key_expr(),
        }
    }
}

/// Errors during entity conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EntityConversionError {
    #[error("missing admin space")]
    MissingAdminSpace,
    #[error("missing Zenoh id")]
    MissingZId,
    #[error("missing node id")]
    MissingNodeId,
    #[error("missing entity id")]
    MissingEntityId,
    #[error("missing entity kind")]
    MissingEntityKind,
    #[error("missing namespace")]
    MissingNamespace,
    #[error("missing node name")]
    MissingNodeName,
    #[error("missing topic name")]
    MissingTopicName,
    #[error("missing topic type")]
    MissingTopicType,
    #[error("missing topic hash")]
    MissingTopicHash,
    #[error("missing topic QoS")]
    MissingTopicQoS,
    #[error("failed to parse liveliness field")]
    ParsingError,
    #[error("failed to decode QoS")]
    QosDecodeError(#[from] QosDecodeError),
}

#[cfg(test)]
mod tests {
    use super::{
        ENDPOINT_GLOBAL_ID_SIZE, EndpointEntity, EndpointGlobalId, EndpointKind, Entity,
        EntityKind, NodeEntity, SchemaHash, TypeInfo, normalize_node_namespace,
    };
    use crate::qos::QosProfile;

    fn node(namespace: &str) -> NodeEntity {
        NodeEntity::new(
            Default::default(),
            1,
            "node".to_string(),
            namespace.to_string(),
        )
    }

    fn endpoint(kind: EndpointKind) -> EndpointEntity {
        EndpointEntity {
            id: 2,
            node: node("/robot"),
            kind,
            topic: "/topic".to_string(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: QosProfile::default(),
        }
    }

    fn endpoint_with_id(id: usize) -> EndpointEntity {
        EndpointEntity {
            id,
            node: node("/robot"),
            kind: EndpointKind::Publisher,
            topic: "/topic".to_string(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: QosProfile::default(),
        }
    }

    #[test]
    fn fully_qualified_name_formats_root_namespace() {
        assert_eq!(node("/").fully_qualified_name(), "/node");
    }

    #[test]
    fn fully_qualified_name_formats_empty_namespace_as_root() {
        assert_eq!(node("").fully_qualified_name(), "/node");
    }

    #[test]
    fn fully_qualified_name_inserts_separator_after_non_root_namespace() {
        assert_eq!(node("/robot").fully_qualified_name(), "/robot/node");
    }

    #[test]
    fn fully_qualified_name_prefixes_bare_namespace() {
        assert_eq!(node("robot").fully_qualified_name(), "/robot/node");
    }

    #[test]
    fn normalize_node_namespace_formats_root_as_empty() {
        assert_eq!(normalize_node_namespace("/"), "");
    }

    #[test]
    fn normalize_node_namespace_keeps_non_root_namespace() {
        assert_eq!(normalize_node_namespace("/robot"), "/robot");
    }

    #[test]
    fn node_key_uses_normalized_namespace_and_name() {
        assert_eq!(node("/").key(), ("".to_string(), "node".to_string()));
        assert_eq!(
            node("/robot").key(),
            ("/robot".to_string(), "node".to_string())
        );
    }

    #[test]
    fn entity_kind_returns_node_for_node_entity() {
        assert_eq!(Entity::Node(node("/")).kind(), EntityKind::Node);
    }

    #[test]
    fn entity_kind_returns_endpoint_kind_for_endpoint_entity() {
        for (endpoint_kind, entity_kind) in [
            (EndpointKind::Publisher, EntityKind::Publisher),
            (EndpointKind::Subscription, EntityKind::Subscription),
            (EndpointKind::Service, EntityKind::Service),
            (EndpointKind::Client, EntityKind::Client),
        ] {
            assert_eq!(
                Entity::Endpoint(endpoint(endpoint_kind)).kind(),
                entity_kind
            );
        }
    }

    #[test]
    fn entity_as_endpoint_projects_endpoint_variant() {
        let endpoint = endpoint(EndpointKind::Publisher);
        let entity = Entity::Endpoint(endpoint.clone());

        assert_eq!(entity.as_endpoint(), Some(&endpoint));
    }

    #[test]
    fn entity_as_endpoint_returns_none_for_node_variant() {
        assert!(Entity::Node(node("/")).as_endpoint().is_none());
    }

    #[test]
    fn node_liveliness_key_expr_uses_existing_native_format() {
        let node = node("/robot");

        let key_expr = node.liveliness_key_expr().unwrap().to_string();

        assert_eq!(key_expr, format!("@ros_z/{}/1/1/NN/%robot/node", node.z_id));
    }

    #[test]
    fn entity_liveliness_key_expr_delegates_to_variant() {
        let endpoint = endpoint(EndpointKind::Publisher);
        let expected = endpoint.liveliness_key_expr().unwrap();
        let entity = Entity::Endpoint(endpoint);

        assert_eq!(entity.liveliness_key_expr().unwrap(), expected);
    }

    #[test]
    fn endpoint_global_id_roundtrips_bytes() {
        let bytes = [7u8; ENDPOINT_GLOBAL_ID_SIZE];

        let id = EndpointGlobalId::new(bytes);

        assert_eq!(id.as_bytes(), &bytes);
        assert_eq!(id.into_bytes(), bytes);
        assert_eq!(EndpointGlobalId::from(bytes).as_bytes(), &bytes);
        let roundtrip: [u8; ENDPOINT_GLOBAL_ID_SIZE] = EndpointGlobalId::from(bytes).into();
        assert_eq!(roundtrip, bytes);
    }

    #[test]
    fn endpoint_global_id_displays_lowercase_fixed_width_hex() {
        let id = EndpointGlobalId::from([
            0x00, 0x01, 0x0a, 0x0f, 0x10, 0x2b, 0x7f, 0x80, 0xab, 0xcd, 0xef, 0xf0, 0x12, 0x34,
            0x56, 0x78,
        ]);

        assert_eq!(id.to_string(), "00010a0f102b7f80abcdeff012345678");
    }

    #[test]
    fn endpoint_global_id_zero_is_all_zero_bytes() {
        assert_eq!(
            EndpointGlobalId::ZERO.as_bytes(),
            &[0u8; ENDPOINT_GLOBAL_ID_SIZE]
        );
    }

    #[test]
    fn endpoint_global_id_is_stable_for_same_node_zenoh_id_and_endpoint_local_id() {
        let node = node("/");
        let endpoint = EndpointEntity {
            node: node.clone(),
            ..endpoint_with_id(7)
        };
        let matching_endpoint = EndpointEntity {
            node,
            ..endpoint_with_id(7)
        };

        assert_eq!(
            EndpointGlobalId::from(&endpoint),
            EndpointGlobalId::from(&matching_endpoint)
        );
    }

    #[test]
    fn endpoint_global_id_changes_when_endpoint_local_id_changes() {
        let node = node("/");
        let endpoint = EndpointEntity {
            node: node.clone(),
            ..endpoint_with_id(7)
        };
        let different_endpoint = EndpointEntity {
            node,
            ..endpoint_with_id(8)
        };

        assert_ne!(
            EndpointGlobalId::from(&endpoint),
            EndpointGlobalId::from(&different_endpoint)
        );
    }
}
