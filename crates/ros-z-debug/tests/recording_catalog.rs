use std::sync::Arc;

use ros_z::{Message, time::Time};
use ros_z_debug::query::{TopicInfo, remote};

#[derive(serde::Serialize, serde::Deserialize, ros_z::Message)]
struct Stamped {
    time: Time,
    value: u32,
}

#[test]
fn catalog_wire_roundtrip_preserves_integer_precision_and_schema() {
    let time = Time::from_nanos(1_790_000_000_123_456_789);
    let schema = Arc::new(Stamped::schema());
    let response = remote::Response::Catalog(remote::RecordingInfo {
        instance: "test".into(),
        name: "test.mcap".into(),
        start: time,
        end: time,
        topics: vec![TopicInfo {
            name: "/images".into(),
            type_info: Stamped::type_info(),
            schema: Some(Arc::clone(&schema)),
            error: None,
        }],
    });
    let remote::Response::Catalog(decoded) =
        remote::decode(&remote::encode(&response).unwrap()).unwrap()
    else {
        panic!("catalog expected")
    };
    assert_eq!(decoded.start, time);
    assert_eq!(decoded.end, time);
    assert_eq!(decoded.topics[0].type_info, Stamped::type_info());
    assert_eq!(
        serde_json::to_value(decoded.topics[0].schema.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&schema).unwrap()
    );
    assert!(zenoh::key_expr::OwnedKeyExpr::try_from(remote::query_key("/")).is_ok());
}
