//! Tests the native formatter: liveliness key expressions can be parsed back
//! to the remaining entity fields, namespaced topics are preserved, all entity
//! kinds round-trip, and invalid key expressions return Err rather than panicking.

use ros_z_protocol::{
    entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, SchemaHash, TypeInfo},
    format,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
};
use zenoh::session::ZenohId;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_node() -> NodeEntity {
    NodeEntity {
        z_id: ZenohId::default(),
        id: 1,
        name: "test_node".to_string(),
        namespace: "/".to_string(),
        enclave: "/".to_string(),
    }
}

fn endpoint_entity(kind: EndpointKind, topic: &str) -> EndpointEntity {
    EndpointEntity {
        id: 42,
        node: Some(default_node()),
        kind,
        topic: topic.to_string(),
        type_info: Some(TypeInfo {
            name: "std_msgs::String".to_string(),
            hash: Some(SchemaHash::zero()),
        }),
        qos: QosProfile {
            reliability: QosReliability::Reliable,
            durability: QosDurability::Volatile,
            history: QosHistory::KeepLast(10),
            ..Default::default()
        },
    }
}

fn parse_liveliness(ke_str: &str) -> zenoh::Result<ros_z_protocol::entity::Entity> {
    let ke: zenoh::key_expr::KeyExpr<'static> = ke_str.to_string().try_into()?;
    format::parse_liveliness(&ke)
}

fn native_endpoint_liveliness(kind: EndpointKind) -> ros_z_protocol::entity::LivelinessKE {
    let zid: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
    let node = NodeEntity {
        z_id: zid,
        id: 1,
        name: "talker".to_string(),
        namespace: String::new(),
        enclave: String::new(),
    };
    let entity = EndpointEntity {
        id: 2,
        node: Some(node),
        kind,
        topic: "/chatter".to_string(),
        type_info: Some(TypeInfo::new("std_msgs::String", None)),
        qos: QosProfile::default(),
    };

    format::liveliness_key_expr(&entity, &zid).unwrap()
}

#[test]
fn parse_native_publisher_liveliness() {
    let key_expr = native_endpoint_liveliness(EndpointKind::Publisher);

    let parsed = format::parse_liveliness(&key_expr).unwrap();

    assert!(
        matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Publisher && endpoint.topic == "/chatter")
    );
}

#[test]
fn parse_native_subscriber_liveliness() {
    let key_expr = native_endpoint_liveliness(EndpointKind::Subscription);

    let parsed = format::parse_liveliness(&key_expr).unwrap();
    assert!(
        matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Subscription)
    );
}

#[test]
fn parse_native_service_liveliness() {
    let key_expr = native_endpoint_liveliness(EndpointKind::Service);

    let parsed = format::parse_liveliness(&key_expr).unwrap();
    assert!(matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Service));
}

#[test]
fn parse_native_client_liveliness() {
    let key_expr = native_endpoint_liveliness(EndpointKind::Client);

    let parsed = format::parse_liveliness(&key_expr).unwrap();
    assert!(matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Client));
}

#[test]
fn parse_native_node_liveliness() {
    let zid: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
    let node = NodeEntity {
        z_id: zid,
        id: 4,
        name: "node".to_string(),
        namespace: "/ns".to_string(),
        enclave: String::new(),
    };
    let key_expr = format::node_liveliness_key_expr(&node).unwrap();

    let parsed = format::parse_liveliness(&key_expr).unwrap();

    assert!(
        matches!(parsed, Entity::Node(node) if node.z_id == zid && node.name == "node" && node.namespace == "/ns")
    );
}

#[test]
fn reject_ros2_liveliness_prefix() {
    let key_expr: zenoh::key_expr::KeyExpr<'static> = concat!(
        "@ros2",
        "_lv/0/1234567890abcdef1234567890abcdef/1/1/MP/%/%/talker/chatter/std_msgs::String/EMPTY_SCHEMA_HASH/Q"
    )
    .try_into()
    .unwrap();

    assert!(format::parse_liveliness(&key_expr).is_err());
}

#[test]
fn missing_type_info_uses_endpoint_neutral_placeholders() {
    let mut entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    entity.type_info = None;

    let topic_key = format::topic_key_expr(&entity).unwrap().to_string();
    assert!(topic_key.contains("EMPTY_TYPE_NAME/EMPTY_SCHEMA_HASH"));

    let liveliness = format::liveliness_key_expr(&entity, &ZenohId::default())
        .unwrap()
        .to_string();
    assert!(liveliness.contains("EMPTY_TYPE_NAME/EMPTY_SCHEMA_HASH"));

    let parsed = parse_liveliness(&liveliness).unwrap();
    match parsed {
        Entity::Endpoint(endpoint) => assert_eq!(endpoint.type_info, None),
        Entity::Node(_) => panic!("expected endpoint"),
    }
}

#[test]
fn qos_encode_decode_round_trips_profile() {
    let qos = QosProfile::default();
    let encoded = format::encode_qos(&qos);
    let decoded = format::decode_qos(&encoded).expect("decode qos");

    assert_eq!(decoded, qos);
}

// ---------------------------------------------------------------------------
// Roundtrip: format then parse returns original fields
// ---------------------------------------------------------------------------

#[test]
fn test_publisher_liveliness_roundtrip() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).expect("liveliness_key_expr");

    let parsed = format::parse_liveliness(&ke).expect("parse_liveliness");

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(
            ep.node.as_ref().unwrap().z_id,
            entity.node.as_ref().unwrap().z_id
        );
        assert_eq!(
            ep.node.as_ref().unwrap().id,
            entity.node.as_ref().unwrap().id
        );
        assert_eq!(
            ep.node.as_ref().unwrap().name,
            entity.node.as_ref().unwrap().name
        );
        assert_eq!(ep.node.as_ref().unwrap().namespace, String::new());
        assert_eq!(ep.kind, EndpointKind::Publisher);
        assert!(
            ep.topic.contains("chatter"),
            "unexpected topic: {}",
            ep.topic
        );
    } else {
        panic!("expected Endpoint entity");
    }
}

#[test]
fn test_subscription_liveliness_roundtrip() {
    let entity = endpoint_entity(EndpointKind::Subscription, "/sensor/data");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(ep.kind, EndpointKind::Subscription);
    } else {
        panic!("expected Endpoint entity");
    }
}

#[test]
fn test_service_liveliness_roundtrip() {
    let entity = endpoint_entity(EndpointKind::Service, "/add_two_ints");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(ep.kind, EndpointKind::Service);
    } else {
        panic!("expected Endpoint entity");
    }
}

#[test]
fn test_client_liveliness_roundtrip() {
    let entity = endpoint_entity(EndpointKind::Client, "/add_two_ints");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(ep.kind, EndpointKind::Client);
    } else {
        panic!("expected Endpoint entity");
    }
}

#[test]
fn test_type_info_without_hash_roundtrip() {
    let mut entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    entity.type_info = Some(TypeInfo::new("test_action::FeedbackMessage", None));
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(
            ep.type_info,
            Some(TypeInfo::new("test_action::FeedbackMessage", None))
        );
    } else {
        panic!("expected Endpoint entity");
    }
}

// ---------------------------------------------------------------------------
// Node liveliness roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_node_liveliness_roundtrip() {
    let z_id = ZenohId::default();
    let node = NodeEntity {
        z_id,
        id: 5,
        name: "my_node".to_string(),
        namespace: "/my_ns".to_string(),
        enclave: "/".to_string(),
    };

    let ke = format::node_liveliness_key_expr(&node).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Node(n) = parsed {
        assert_eq!(n.z_id, z_id);
        assert_eq!(n.id, 5);
        assert_eq!(n.name, "my_node");
        assert_eq!(n.namespace, "/my_ns");
    } else {
        panic!("expected Node entity");
    }
}

// ---------------------------------------------------------------------------
// Topic names with namespaces
// ---------------------------------------------------------------------------

#[test]
fn test_topic_with_namespace() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/ns/topic");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        // "ns/topic" after leading slash stripped
        assert!(
            ep.topic.contains("ns") && ep.topic.contains("topic"),
            "unexpected topic: {}",
            ep.topic
        );
    } else {
        panic!("expected Endpoint");
    }
}

#[test]
fn test_topic_without_namespace() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/topic");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert!(ep.topic.contains("topic"), "unexpected topic: {}", ep.topic);
    } else {
        panic!("expected Endpoint");
    }
}

// ---------------------------------------------------------------------------
// Topic key expression format checks
// ---------------------------------------------------------------------------

#[test]
fn test_topic_key_expr_format() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    let ke = format::topic_key_expr(&entity).unwrap();
    let ke_str = ke.to_string();
    assert!(
        ke_str.starts_with("rt/"),
        "expected native topic prefix: {ke_str}"
    );
    assert!(ke_str.contains("chatter"), "expected topic: {}", ke_str);
}

#[test]
fn test_topic_key_expr_preserves_internal_slashes() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/ns/topic");
    let ke = format::topic_key_expr(&entity).unwrap();
    let ke_str = ke.to_string();
    // Internal slashes must be preserved (not mangled)
    assert!(
        ke_str.contains("ns/topic"),
        "expected preserved internal slashes: {}",
        ke_str
    );
}

// ---------------------------------------------------------------------------
// Invalid key expressions return Err (not panic)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_empty_key_expr_is_err() {
    // An empty or single-segment key expression has no admin prefix
    let result = parse_liveliness("not_admin_space/0/1/2/3/NN/%/%/node");
    assert!(result.is_err(), "expected Err for invalid admin space");
}

#[test]
fn test_parse_truncated_key_expr_is_err() {
    // Missing fields after admin space
    let result =
        parse_liveliness("@ros_z/0000000000000000000000000000000000000000000000000000000000000000");
    assert!(result.is_err(), "expected Err for truncated key expression");
}

#[test]
fn test_parse_invalid_zid_is_err() {
    // z_id that cannot be parsed as a Zenoh id
    let result = parse_liveliness("@ros_z/notanumber/0/0/NN/%/node");
    assert!(result.is_err(), "expected Err for invalid z_id");
}

#[test]
fn test_parse_node_with_trailing_segment_is_err() {
    let result = parse_liveliness("@ros_z/1234567890abcdef1234567890abcdef/1/1/NN/%/node/extra");
    assert!(
        result.is_err(),
        "expected Err for node liveliness with trailing segment"
    );
}

#[test]
fn test_parse_endpoint_with_trailing_segment_is_err() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    let ke = format::liveliness_key_expr(&entity, &ZenohId::default()).unwrap();
    let result = parse_liveliness(&format!("{}/extra", ke.as_str()));
    assert!(
        result.is_err(),
        "expected Err for endpoint liveliness with trailing segment"
    );
}
