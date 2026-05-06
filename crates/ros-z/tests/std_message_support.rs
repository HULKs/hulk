use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::{Range, RangeInclusive},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ros_z::{
    Message, MessageCodec,
    dynamic::{PrimitiveType, RuntimeDynamicEnumPayload, RuntimeFieldSchema, TypeShape},
};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct StdEnvelope {
    duration: Duration,
    system_time: SystemTime,
    count: usize,
    range: Range<f32>,
    inclusive: RangeInclusive<f32>,
    address: SocketAddr,
    contacts: HashMap<SocketAddr, SystemTime>,
}

fn struct_fields(schema: &ros_z::dynamic::Schema) -> &[RuntimeFieldSchema] {
    let TypeShape::Struct { fields, .. } = schema.as_ref() else {
        panic!("expected struct schema, got {schema:?}");
    };
    fields
}

fn field<'a>(schema: &'a ros_z::dynamic::Schema, name: &str) -> &'a RuntimeFieldSchema {
    struct_fields(schema)
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn named_struct_fields<'a>(
    schema: &'a ros_z::dynamic::Schema,
    expected_name: &str,
) -> &'a [RuntimeFieldSchema] {
    let TypeShape::Struct { name, fields } = schema.as_ref() else {
        panic!("expected named struct schema, got {schema:?}");
    };
    assert_eq!(name.as_str(), expected_name);
    fields
}

fn assert_socket_addr_inner_schema(
    schema: &ros_z::dynamic::Schema,
    expected_name: &str,
    expected_ip_length: usize,
) {
    let fields = named_struct_fields(schema, expected_name);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "ip");
    match fields[0].schema.as_ref() {
        TypeShape::Sequence { element, length } => {
            assert_eq!(
                *length,
                ros_z::dynamic::SequenceLength::Fixed(expected_ip_length)
            );
            assert_primitive(element, PrimitiveType::U8);
        }
        other => panic!("expected fixed IP octet sequence, got {other:?}"),
    }
    assert_eq!(fields[1].name, "port");
    assert_primitive(&fields[1].schema, PrimitiveType::U16);
}

fn assert_socket_addr_schema(schema: &ros_z::dynamic::Schema) {
    let TypeShape::Enum { name, variants } = schema.as_ref() else {
        panic!("expected SocketAddr enum schema");
    };

    assert_eq!(name.as_str(), "std::net::SocketAddr");
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].name, "V4");
    assert_eq!(variants[1].name, "V6");

    let RuntimeDynamicEnumPayload::Newtype(v4_schema) = &variants[0].payload else {
        panic!("expected V4 newtype payload");
    };
    assert_socket_addr_inner_schema(v4_schema, "std::net::SocketAddrV4", 4);

    let RuntimeDynamicEnumPayload::Newtype(v6_schema) = &variants[1].payload else {
        panic!("expected V6 newtype payload");
    };
    assert_socket_addr_inner_schema(v6_schema, "std::net::SocketAddrV6", 16);
}

fn assert_primitive(schema: &ros_z::dynamic::Schema, expected: PrimitiveType) {
    assert!(
        matches!(schema.as_ref(), TypeShape::Primitive(actual) if *actual == expected),
        "expected primitive {expected:?}, got {schema:?}"
    );
}

#[test]
fn std_duration_schema_uses_serde_field_names() {
    assert_eq!(Duration::type_name(), "std::time::Duration");
    let schema = Duration::schema();
    let fields = named_struct_fields(&schema, "std::time::Duration");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "secs");
    assert_primitive(&fields[0].schema, PrimitiveType::U64);
    assert_eq!(fields[1].name, "nanos");
    assert_primitive(&fields[1].schema, PrimitiveType::U32);
}

#[test]
fn std_system_time_schema_uses_serde_field_names() {
    assert_eq!(SystemTime::type_name(), "std::time::SystemTime");
    let schema = SystemTime::schema();
    let fields = named_struct_fields(&schema, "std::time::SystemTime");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "secs_since_epoch");
    assert_primitive(&fields[0].schema, PrimitiveType::U64);
    assert_eq!(fields[1].name, "nanos_since_epoch");
    assert_primitive(&fields[1].schema, PrimitiveType::U32);
}

#[test]
fn usize_schema_is_u64_primitive() {
    assert_eq!(usize::type_name(), "usize");
    assert_primitive(&usize::schema(), PrimitiveType::U64);
}

#[test]
fn range_schemas_use_start_and_end_fields() {
    assert_eq!(Range::<f32>::type_name(), "Range<f32>");
    let range_schema = Range::<f32>::schema();
    let range_fields = named_struct_fields(&range_schema, "Range<f32>");
    assert_eq!(range_fields.len(), 2);
    assert_eq!(range_fields[0].name, "start");
    assert_primitive(&range_fields[0].schema, PrimitiveType::F32);
    assert_eq!(range_fields[1].name, "end");
    assert_primitive(&range_fields[1].schema, PrimitiveType::F32);

    assert_eq!(RangeInclusive::<f32>::type_name(), "RangeInclusive<f32>");
    let inclusive_schema = RangeInclusive::<f32>::schema();
    let inclusive_fields = named_struct_fields(&inclusive_schema, "RangeInclusive<f32>");
    assert_eq!(inclusive_fields.len(), 2);
    assert_eq!(inclusive_fields[0].name, "start");
    assert_primitive(&inclusive_fields[0].schema, PrimitiveType::F32);
    assert_eq!(inclusive_fields[1].name, "end");
    assert_primitive(&inclusive_fields[1].schema, PrimitiveType::F32);
}

#[test]
fn socket_addr_schema_matches_serde_cdr_enum_shape() {
    assert_eq!(SocketAddr::type_name(), "std::net::SocketAddr");
    assert_socket_addr_schema(&SocketAddr::schema());
}

#[test]
fn derived_message_can_contain_all_core_std_types() {
    let schema = StdEnvelope::schema();

    assert!(matches!(
        field(&schema, "duration").schema.as_ref(),
        TypeShape::Struct { .. }
    ));
    assert!(matches!(
        field(&schema, "system_time").schema.as_ref(),
        TypeShape::Struct { .. }
    ));
    assert!(matches!(
        field(&schema, "count").schema.as_ref(),
        TypeShape::Primitive(PrimitiveType::U64)
    ));
    assert!(matches!(
        field(&schema, "range").schema.as_ref(),
        TypeShape::Struct { .. }
    ));
    assert!(matches!(
        field(&schema, "inclusive").schema.as_ref(),
        TypeShape::Struct { .. }
    ));
    assert!(matches!(
        field(&schema, "address").schema.as_ref(),
        TypeShape::Enum { .. }
    ));

    let contacts = field(&schema, "contacts");
    let TypeShape::Map { key, value } = contacts.schema.as_ref() else {
        panic!("expected contacts map schema, got {contacts:?}");
    };
    assert_socket_addr_schema(key);
    assert!(matches!(value.as_ref(), TypeShape::Struct { .. }));
}

#[test]
fn std_envelope_round_trips_through_cdr() {
    let mut contacts = HashMap::new();
    contacts.insert(
        "127.0.0.1:3838".parse::<SocketAddr>().unwrap(),
        UNIX_EPOCH + Duration::new(123, 456),
    );
    contacts.insert(
        "[::1]:3839".parse::<SocketAddr>().unwrap(),
        UNIX_EPOCH + Duration::new(789, 123),
    );

    let original = StdEnvelope {
        duration: Duration::new(5, 6),
        system_time: UNIX_EPOCH + Duration::new(7, 8),
        count: 42,
        range: 1.0..2.0,
        inclusive: 3.0..=4.0,
        address: "[::1]:3838".parse().unwrap(),
        contacts,
    };

    let encoded = <StdEnvelope as Message>::Codec::encode(&original).unwrap();
    let decoded = <StdEnvelope as Message>::Codec::decode(&encoded.payload.contiguous()).unwrap();

    assert_eq!(decoded, original);
}
