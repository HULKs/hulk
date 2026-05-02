//! Native ros-z key expression format.
//!
//! Key expression formats:
//! - Topic: `rt/<topic>/<type>/<hash>`
//! - Liveliness: `@ros_z/<zid>/<nid>/<eid>/<kind>/<ns>/<name>[/<topic>/<type>/<hash>/<qos>]`

use alloc::string::{String, ToString};
use zenoh::{Result, key_expr::KeyExpr, session::ZenohId};

use crate::{
    entity::{
        EndpointEntity, EndpointKind, Entity, EntityConversionError, EntityKind, LivelinessKE,
        NodeEntity, SchemaHash, TopicKE, TypeInfo,
    },
    qos::QosProfile,
};

pub const ADMIN_SPACE: &str = "@ros_z";
pub const EMPTY_PLACEHOLDER: &str = "%";
pub const EMPTY_TYPE_NAME: &str = "EMPTY_TYPE_NAME";
pub const EMPTY_SCHEMA_HASH: &str = "EMPTY_SCHEMA_HASH";

const ESCAPE_CHAR: char = '%';

fn format_type_hash(hash: Option<&SchemaHash>) -> String {
    hash.map(SchemaHash::to_hash_string)
        .unwrap_or_else(|| EMPTY_SCHEMA_HASH.to_string())
}

fn stripped_topic(topic: &str) -> &str {
    let topic = topic.strip_prefix('/').unwrap_or(topic);
    topic.strip_suffix('/').unwrap_or(topic)
}

pub fn topic_key_expr(entity: &EndpointEntity) -> Result<TopicKE> {
    let EndpointEntity {
        node: Some(_),
        topic,
        type_info,
        ..
    } = entity
    else {
        return Err(zenoh::Error::from(
            "native endpoint keys require node identity",
        ));
    };

    let topic = stripped_topic(topic);
    let type_info =
        type_info
            .as_ref()
            .map_or(format!("{EMPTY_TYPE_NAME}/{EMPTY_SCHEMA_HASH}"), |info| {
                let type_name = demangle_name(&info.name);
                let type_hash = demangle_name(&format_type_hash(info.hash.as_ref()));
                format!("{type_name}/{type_hash}")
            });

    Ok(TopicKE::new(format!("rt/{topic}/{type_info}").try_into()?))
}

pub fn liveliness_key_expr(entity: &EndpointEntity, _zid: &ZenohId) -> Result<LivelinessKE> {
    let EndpointEntity {
        id,
        node:
            Some(NodeEntity {
                z_id,
                id: node_id,
                name: node_name,
                namespace: node_namespace,
                ..
            }),
        kind,
        topic: topic_name,
        type_info,
        qos,
    } = entity
    else {
        return Err(zenoh::Error::from(
            "native liveliness requires node identity",
        ));
    };

    let node_namespace = if node_namespace.is_empty() {
        EMPTY_PLACEHOLDER.to_string()
    } else {
        mangle_name(node_namespace)
    };
    let node_name = mangle_name(node_name);
    let topic_name = mangle_name(topic_name.strip_suffix('/').unwrap_or(topic_name));
    let type_info_str =
        type_info
            .as_ref()
            .map_or(format!("{EMPTY_TYPE_NAME}/{EMPTY_SCHEMA_HASH}"), |info| {
                format!(
                    "{}/{}",
                    mangle_name(&info.name),
                    format_type_hash(info.hash.as_ref())
                )
            });
    let qos_str = qos.encode();

    let ke = format!(
        "{ADMIN_SPACE}/{z_id}/{node_id}/{id}/{kind}/{node_namespace}/{node_name}/{topic_name}/{type_info_str}/{qos_str}"
    );

    Ok(LivelinessKE::new(ke.try_into()?))
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

    Ok(LivelinessKE::new(
        format!("{ADMIN_SPACE}/{z_id}/{id}/{id}/NN/{namespace}/{name}").try_into()?,
    ))
}

pub fn parse_liveliness(ke: &KeyExpr) -> Result<Entity> {
    use EntityConversionError::*;

    let mut iter = ke.split('/');

    let admin = iter.next().ok_or(MissingAdminSpace)?;
    if admin != ADMIN_SPACE {
        return Err(zenoh::Error::from(MissingAdminSpace));
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
        enclave: String::new(),
    };

    let entity = match entity_kind {
        EntityKind::Node => Entity::Node(node),
        _ => {
            let topic_name = demangle_name(iter.next().ok_or(MissingTopicName)?);
            let topic_type = iter.next().ok_or(MissingTopicType)?;
            let topic_hash = iter.next().ok_or(MissingTopicHash)?;

            let type_info = match (topic_type, topic_hash) {
                (EMPTY_TYPE_NAME, EMPTY_SCHEMA_HASH) => None,
                (EMPTY_TYPE_NAME, _) => None,
                (topic_type, EMPTY_SCHEMA_HASH) => {
                    Some(TypeInfo::new(&demangle_name(topic_type), None))
                }
                (topic_type, topic_hash) => Some(TypeInfo::new(
                    &demangle_name(topic_type),
                    Some(SchemaHash::from_hash_string(topic_hash).map_err(|_| ParsingError)?),
                )),
            };

            let qos =
                QosProfile::decode(iter.next().ok_or(MissingTopicQoS)?).map_err(QosDecodeError)?;

            Entity::Endpoint(EndpointEntity {
                id: entity_id,
                node: Some(node),
                kind: EndpointKind::try_from(entity_kind).map_err(|_| ParsingError)?,
                topic: topic_name,
                type_info,
                qos,
            })
        }
    };

    if iter.next().is_some() {
        return Err(zenoh::Error::from(ParsingError));
    }

    Ok(entity)
}

pub fn encode_qos(qos: &QosProfile) -> String {
    qos.encode()
}

pub fn decode_qos(s: &str) -> Result<QosProfile> {
    QosProfile::decode(s)
        .map_err(|error| zenoh::Error::from(format!("QoS decode error: {error:?}")))
}

fn mangle_name(name: &str) -> String {
    name.replace('/', &ESCAPE_CHAR.to_string())
}

fn demangle_name(name: &str) -> String {
    name.replace(ESCAPE_CHAR, "/")
}
