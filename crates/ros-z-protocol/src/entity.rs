//! ros-z entity types for key expression generation.

use core::{fmt::Display, ops::Deref};
pub use ros_z_schema::SchemaHash;
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
        if self.namespace == "/" {
            format!("/{}", self.name)
        } else {
            format!("{}/{}", self.namespace, self.name)
        }
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
}

/// Generic ros-z entity (node or endpoint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entity {
    Node(NodeEntity),
    Endpoint(EndpointEntity),
}

/// Errors during entity conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityConversionError {
    MissingAdminSpace,
    MissingZId,
    MissingNodeId,
    MissingEntityId,
    MissingEntityKind,
    MissingNamespace,
    MissingNodeName,
    MissingTopicName,
    MissingTopicType,
    MissingTopicHash,
    MissingTopicQoS,
    ParsingError,
    QosDecodeError(QosDecodeError),
}

impl Display for EntityConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for EntityConversionError {}

#[cfg(test)]
mod tests {
    use super::NodeEntity;

    fn node(namespace: &str) -> NodeEntity {
        NodeEntity::new(
            Default::default(),
            1,
            "node".to_string(),
            namespace.to_string(),
        )
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
}
