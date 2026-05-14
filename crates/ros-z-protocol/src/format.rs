//! Native ros-z key expression format.
//!
//! Key expression formats:
//! - Topic: `rt/<topic>/<type>/<hash>`
//! - Liveliness: `@ros_z/<zid>/<nid>/<eid>/<kind>/<ns>/<name>[/<topic>/<type>/<hash>/<qos>]`

use zenoh::{key_expr::KeyExpr, session::ZenohId};

use crate::{
    entity::{
        EndpointEntity, EndpointKind, Entity, EntityConversionError, EntityKind, LivelinessKE,
        NodeEntity, SchemaHash, TopicKE, TypeInfo,
    },
    error::{ProtocolError, Result},
    qos::QosProfile,
};

pub const ADMIN_SPACE: &str = "@ros_z";
pub const EMPTY_PLACEHOLDER: &str = "%";

const ESCAPE_CHAR: char = '%';

fn key_expr(expression: String) -> Result<zenoh::key_expr::KeyExpr<'static>> {
    expression
        .clone()
        .try_into()
        .map_err(|source| ProtocolError::InvalidKeyExpression { expression, source })
}

fn stripped_topic(topic: &str) -> &str {
    let topic = topic.strip_prefix('/').unwrap_or(topic);
    topic.strip_suffix('/').unwrap_or(topic)
}

pub fn topic_key_expr(entity: &EndpointEntity) -> Result<TopicKE> {
    let EndpointEntity {
        topic, type_info, ..
    } = entity;

    let topic = stripped_topic(topic);
    let type_name = demangle_name(&type_info.name);
    let type_hash = demangle_name(&type_info.hash.to_hash_string());

    Ok(TopicKE::new(key_expr(format!(
        "rt/{topic}/{type_name}/{type_hash}"
    ))?))
}

pub fn liveliness_key_expr(entity: &EndpointEntity, _zid: &ZenohId) -> Result<LivelinessKE> {
    let EndpointEntity {
        id,
        node:
            NodeEntity {
                z_id,
                id: node_id,
                name: node_name,
                namespace: node_namespace,
                ..
            },
        kind,
        topic: topic_name,
        type_info,
        qos,
    } = entity;

    let node_namespace = if node_namespace.is_empty() {
        EMPTY_PLACEHOLDER.to_string()
    } else {
        mangle_name(node_namespace)
    };
    let node_name = mangle_name(node_name);
    let topic_name = mangle_name(topic_name.strip_suffix('/').unwrap_or(topic_name));
    let type_name = mangle_name(&type_info.name);
    let type_hash = type_info.hash.to_hash_string();
    let qos_str = qos.encode();

    let ke = format!(
        "{ADMIN_SPACE}/{z_id}/{node_id}/{id}/{kind}/{node_namespace}/{node_name}/{topic_name}/{type_name}/{type_hash}/{qos_str}"
    );

    Ok(LivelinessKE::new(key_expr(ke)?))
}

pub fn node_liveliness_key_expr(entity: &NodeEntity) -> Result<LivelinessKE> {
    let NodeEntity {
        z_id,
        id,
        name,
        namespace,
        ..
    } = entity;

    let namespace = if namespace.is_empty() {
        EMPTY_PLACEHOLDER.to_string()
    } else {
        mangle_name(namespace)
    };
    let name = mangle_name(name);

    let ke = format!("{ADMIN_SPACE}/{z_id}/{id}/{id}/NN/{namespace}/{name}");
    Ok(LivelinessKE::new(key_expr(ke)?))
}

fn parse_liveliness_inner(ke: &KeyExpr) -> std::result::Result<Entity, EntityConversionError> {
    use EntityConversionError::*;

    let mut iter = ke.split('/');

    let admin = iter.next().ok_or(MissingAdminSpace)?;
    if admin != ADMIN_SPACE {
        return Err(MissingAdminSpace);
    }

    let z_id = iter
        .next()
        .ok_or(MissingZId)?
        .parse()
        .map_err(|_| ParsingError)?;
    let node_id = iter
        .next()
        .ok_or(MissingNodeId)?
        .parse()
        .map_err(|_| ParsingError)?;
    let entity_id = iter
        .next()
        .ok_or(MissingEntityId)?
        .parse()
        .map_err(|_| ParsingError)?;
    let entity_kind: EntityKind = iter
        .next()
        .ok_or(MissingEntityKind)?
        .parse()
        .map_err(|_| ParsingError)?;

    let namespace = match iter.next().ok_or(MissingNamespace)? {
        EMPTY_PLACEHOLDER => String::new(),
        value => demangle_name(value),
    };
    let node_name = demangle_name(iter.next().ok_or(MissingNodeName)?);

    let node = NodeEntity {
        z_id,
        id: node_id,
        name: node_name,
        namespace,
    };

    let entity = match entity_kind {
        EntityKind::Node => Entity::Node(node),
        _ => {
            let topic_name = demangle_name(iter.next().ok_or(MissingTopicName)?);
            let topic_type = iter.next().ok_or(MissingTopicType)?;
            let topic_hash = iter.next().ok_or(MissingTopicHash)?;

            let type_info = TypeInfo::new(
                demangle_name(topic_type),
                SchemaHash::from_hash_string(topic_hash).map_err(|_| ParsingError)?,
            );

            let qos =
                QosProfile::decode(iter.next().ok_or(MissingTopicQoS)?).map_err(QosDecodeError)?;

            Entity::Endpoint(EndpointEntity {
                id: entity_id,
                node,
                kind: EndpointKind::try_from(entity_kind).map_err(|_| ParsingError)?,
                topic: topic_name,
                type_info,
                qos,
            })
        }
    };

    if iter.next().is_some() {
        return Err(ParsingError);
    }

    Ok(entity)
}

pub fn parse_liveliness(ke: &KeyExpr) -> Result<Entity> {
    parse_liveliness_inner(ke).map_err(|source| ProtocolError::ParseLiveliness {
        key_expr: ke.to_string(),
        source,
    })
}

fn mangle_name(name: &str) -> String {
    name.replace('/', &ESCAPE_CHAR.to_string())
}

fn demangle_name(name: &str) -> String {
    name.replace(ESCAPE_CHAR, "/")
}
