//! Tests the native formatter: liveliness key expressions can be parsed back
//! to the remaining entity fields, namespaced topics are preserved, all entity
//! kinds round-trip, and invalid key expressions return Err rather than panicking.

use ros_z_protocol::{
    entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, SchemaHash, TypeInfo},
    error::ProtocolError,
    format,
    format::parse_liveliness as parse_liveliness_key,
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
    }
}

fn endpoint_entity(kind: EndpointKind, topic: &str) -> EndpointEntity {
    EndpointEntity {
        id: 42,
        node: default_node(),
        kind,
        topic: topic.to_string(),
        type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
        qos: QosProfile {
            reliability: QosReliability::Reliable,
            durability: QosDurability::Volatile,
            history: QosHistory::KeepLast(10),
            ..Default::default()
        },
    }
}

fn parse_liveliness(
    ke_str: &str,
) -> Result<ros_z_protocol::entity::Entity, Box<dyn std::error::Error + Send + Sync>> {
    let ke: zenoh::key_expr::KeyExpr<'static> = ke_str.to_string().try_into()?;
    Ok(format::parse_liveliness(&ke)?)
}

fn native_endpoint_liveliness(kind: EndpointKind) -> ros_z_protocol::entity::LivelinessKE {
    let zid: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
    let node = NodeEntity {
        z_id: zid,
        id: 1,
        name: "talker".to_string(),
        namespace: String::new(),
    };
    let entity = EndpointEntity {
        id: 2,
        node,
        kind,
        topic: "/chatter".to_string(),
        type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
        qos: QosProfile::default(),
    };

    format::liveliness_key_expr(&entity, &zid).unwrap()
}

#[test]
fn parse_native_endpoint_liveliness_preserves_endpoint_kind_and_topic() {
    for kind in [
        EndpointKind::Publisher,
        EndpointKind::Subscription,
        EndpointKind::Service,
        EndpointKind::Client,
    ] {
        let key_expr = native_endpoint_liveliness(kind);

        let parsed = format::parse_liveliness(&key_expr).unwrap();

        let Entity::Endpoint(endpoint) = parsed else {
            panic!("expected endpoint liveliness for {kind:?}");
        };
        assert_eq!(endpoint.kind, kind);
        assert_eq!(endpoint.topic, "/chatter");
    }
}

#[test]
fn reject_ros2_liveliness_prefix() {
    let key_expr: zenoh::key_expr::KeyExpr<'static> = concat!(
        "@ros2",
        "_lv/0/1234567890abcdef1234567890abcdef/1/1/MP/%/%/talker/chatter/std_msgs::String/0000000000000000000000000000000000000000000000000000000000000000/Q"
    )
    .try_into()
    .unwrap();

    assert!(format::parse_liveliness(&key_expr).is_err());
}

// ---------------------------------------------------------------------------
// Roundtrip: format then parse returns original fields
// ---------------------------------------------------------------------------

#[test]
fn endpoint_liveliness_roundtrip_preserves_public_fields_for_all_endpoint_kinds() {
    for (kind, topic) in [
        (EndpointKind::Publisher, "/chatter"),
        (EndpointKind::Subscription, "/sensor/data"),
        (EndpointKind::Service, "/add_two_ints"),
        (EndpointKind::Client, "/add_two_ints"),
    ] {
        let entity = endpoint_entity(kind, topic);
        let zid = ZenohId::default();

        let ke = format::liveliness_key_expr(&entity, &zid).expect("liveliness_key_expr");
        let parsed = format::parse_liveliness(&ke).expect("parse_liveliness");

        let Entity::Endpoint(ep) = parsed else {
            panic!("expected endpoint liveliness for {kind:?}");
        };
        assert_eq!(ep.kind, kind);
        assert_eq!(ep.topic, topic);
        assert_eq!(ep.type_info, entity.type_info);
        assert_eq!(ep.qos, entity.qos);
        assert_eq!(ep.node.z_id, entity.node.z_id);
        assert_eq!(ep.node.id, entity.node.id);
        assert_eq!(ep.node.name, entity.node.name);
        let expected_namespace = match entity.node.namespace.as_str() {
            "/" => "",
            namespace => namespace,
        };
        assert_eq!(ep.node.namespace, expected_namespace);
    }
}

#[test]
fn test_type_info_with_hash_roundtrip() {
    let hash = SchemaHash([0xab; 32]);
    let mut entity = endpoint_entity(EndpointKind::Publisher, "/chatter");
    entity.type_info = TypeInfo::new("test_action::FeedbackMessage", hash);
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(
            ep.type_info,
            TypeInfo::new("test_action::FeedbackMessage", hash)
        );
    } else {
        panic!("expected Endpoint entity");
    }
}

// ---------------------------------------------------------------------------
// Node liveliness roundtrip
// ---------------------------------------------------------------------------

#[test]
fn node_liveliness_key_uses_native_node_identity_fields() {
    let z_id: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
    let node = NodeEntity::new(z_id, 5, "my_node".to_string(), "/my_ns".to_string());

    let key_expr = format::node_liveliness_key_expr(&node).unwrap().to_string();

    assert_eq!(key_expr, format!("@ros_z/{z_id}/5/5/NN/%my_ns/my_node"));
}

#[test]
fn test_node_liveliness_roundtrip() {
    let z_id = ZenohId::default();
    let node = NodeEntity {
        z_id,
        id: 5,
        name: "my_node".to_string(),
        namespace: "/my_ns".to_string(),
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
fn topic_liveliness_roundtrip_preserves_namespace_segments() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/ns/topic");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(ep.topic, "/ns/topic");
    } else {
        panic!("expected Endpoint");
    }
}

#[test]
fn topic_liveliness_roundtrip_preserves_root_topic() {
    let entity = endpoint_entity(EndpointKind::Publisher, "/topic");
    let zid = ZenohId::default();

    let ke = format::liveliness_key_expr(&entity, &zid).unwrap();
    let parsed = format::parse_liveliness(&ke).unwrap();

    if let Entity::Endpoint(ep) = parsed {
        assert_eq!(ep.topic, "/topic");
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

#[test]
fn parse_liveliness_reports_missing_admin_space() {
    let key_expr: zenoh::key_expr::KeyExpr<'static> = "not_ros_z/abc".try_into().unwrap();

    let error = parse_liveliness_key(&key_expr).expect_err("malformed liveliness should fail");

    assert!(matches!(
        error,
        ProtocolError::ParseLiveliness {
            source: ros_z_protocol::entity::EntityConversionError::MissingAdminSpace,
            ..
        }
    ));
    assert!(
        error
            .to_string()
            .contains("failed to parse ros-z liveliness key")
    );
}

// ---------------------------------------------------------------------------
// Invalid key expressions return Err (not panic)
// ---------------------------------------------------------------------------

#[test]
fn parse_non_ros_z_admin_space_is_err() {
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
