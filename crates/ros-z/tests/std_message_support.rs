use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::{Range, RangeInclusive},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ros_z::Message;
use ros_z::message::{WireDecoder, WireEncoder};
use ros_z_schema::{
    EnumPayloadDef, PrimitiveTypeDef, SequenceLengthDef, TypeDef, TypeDefinition, TypeName,
};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_msgs::RenamedRangeElement")]
struct RenamedRangeElement {
    value: u8,
}

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

fn struct_fields<'a>(
    schema: &'a ros_z_schema::SchemaBundle,
    name: &str,
) -> &'a [ros_z_schema::FieldDef] {
    let name = TypeName::new(name).unwrap();
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&name) else {
        panic!("missing struct definition");
    };
    &definition.fields
}

fn field<'a>(
    schema: &'a ros_z_schema::SchemaBundle,
    root_name: &str,
    name: &str,
) -> &'a ros_z_schema::FieldDef {
    struct_fields(schema, root_name)
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn assert_socket_addr_schema(schema: &ros_z_schema::SchemaBundle) {
    let socket = TypeName::new("std::net::SocketAddr").unwrap();
    let Some(TypeDefinition::Enum(definition)) = schema.definitions.get(&socket) else {
        panic!("missing SocketAddr enum");
    };

    assert_eq!(definition.variants.len(), 2);
    assert_eq!(definition.variants[0].name, "V4");
    assert_eq!(definition.variants[1].name, "V6");

    let EnumPayloadDef::Newtype(TypeDef::Named(v4_name)) = &definition.variants[0].payload else {
        panic!("V4 should be a named newtype payload");
    };
    assert_eq!(v4_name.as_str(), "std::net::SocketAddrV4");
    let v4_fields = struct_fields(schema, "std::net::SocketAddrV4");
    assert_eq!(
        v4_fields[0].shape,
        TypeDef::Sequence {
            element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U8)),
            length: SequenceLengthDef::Fixed(4),
        }
    );
}

#[test]
fn std_duration_schema_uses_serde_field_names() {
    assert_eq!(Duration::type_name(), "std::time::Duration");
    let schema = Duration::schema().unwrap();
    let fields = struct_fields(&schema, "std::time::Duration");

    assert_eq!(fields[0].name, "secs");
    assert_eq!(fields[0].shape, TypeDef::Primitive(PrimitiveTypeDef::U64));
    assert_eq!(fields[1].name, "nanos");
    assert_eq!(fields[1].shape, TypeDef::Primitive(PrimitiveTypeDef::U32));
}

#[test]
fn static_type_names_return_owned_strings() {
    let first = bool::type_name();
    let second = bool::type_name();

    assert_eq!(first, "bool");
    assert_eq!(second, "bool");
    assert_ne!(first.as_ptr(), second.as_ptr());
}

#[test]
fn container_type_names_return_owned_strings() {
    let first = Vec::<u8>::type_name();
    let second = Vec::<u8>::type_name();

    assert_eq!(first, "Vec<u8>");
    assert_eq!(second, "Vec<u8>");
    assert_ne!(first.as_ptr(), second.as_ptr());
}

#[test]
fn std_system_time_schema_uses_serde_field_names() {
    assert_eq!(SystemTime::type_name(), "std::time::SystemTime");
    let schema = SystemTime::schema().unwrap();
    let fields = struct_fields(&schema, "std::time::SystemTime");

    assert_eq!(fields[0].name, "secs_since_epoch");
    assert_eq!(fields[0].shape, TypeDef::Primitive(PrimitiveTypeDef::U64));
    assert_eq!(fields[1].name, "nanos_since_epoch");
    assert_eq!(fields[1].shape, TypeDef::Primitive(PrimitiveTypeDef::U32));
}

#[test]
fn range_schemas_use_start_and_end_fields() {
    let range_schema = Range::<f32>::schema().unwrap();
    let range_fields = struct_fields(&range_schema, "Range<f32>");
    assert_eq!(
        range_fields[0].shape,
        TypeDef::Primitive(PrimitiveTypeDef::F32)
    );
    assert_eq!(
        range_fields[1].shape,
        TypeDef::Primitive(PrimitiveTypeDef::F32)
    );

    let inclusive_schema = RangeInclusive::<f32>::schema().unwrap();
    let inclusive_fields = struct_fields(&inclusive_schema, "RangeInclusive<f32>");
    assert_eq!(
        inclusive_fields[0].shape,
        TypeDef::Primitive(PrimitiveTypeDef::F32)
    );
    assert_eq!(
        inclusive_fields[1].shape,
        TypeDef::Primitive(PrimitiveTypeDef::F32)
    );
}

#[test]
fn range_schema_roots_use_message_type_names() {
    let range_schema = Range::<RenamedRangeElement>::schema().unwrap();
    let range_name = TypeName::new("Range<test_msgs::RenamedRangeElement>").unwrap();
    assert_eq!(range_schema.root, TypeDef::Named(range_name.clone()));
    assert!(range_schema.definitions.contains_key(&range_name));

    let inclusive_schema = RangeInclusive::<RenamedRangeElement>::schema().unwrap();
    let inclusive_name = TypeName::new("RangeInclusive<test_msgs::RenamedRangeElement>").unwrap();
    assert_eq!(
        inclusive_schema.root,
        TypeDef::Named(inclusive_name.clone())
    );
    assert!(inclusive_schema.definitions.contains_key(&inclusive_name));
}

#[test]
fn socket_addr_schema_matches_serde_cdr_enum_shape() {
    let schema = SocketAddr::schema().unwrap();
    assert_socket_addr_schema(&schema);
}

#[test]
fn derived_message_can_contain_all_core_std_types() {
    let schema = StdEnvelope::schema().unwrap();
    let root = StdEnvelope::type_name();

    assert!(matches!(
        field(&schema, &root, "duration").shape,
        TypeDef::Named(_)
    ));
    assert!(matches!(
        field(&schema, &root, "system_time").shape,
        TypeDef::Named(_)
    ));
    assert_eq!(
        field(&schema, &root, "count").shape,
        TypeDef::Primitive(PrimitiveTypeDef::U64)
    );
    assert!(matches!(
        field(&schema, &root, "range").shape,
        TypeDef::Named(_)
    ));
    assert!(matches!(
        field(&schema, &root, "inclusive").shape,
        TypeDef::Named(_)
    ));
    assert!(matches!(
        field(&schema, &root, "address").shape,
        TypeDef::Named(_)
    ));
    assert!(matches!(
        field(&schema, &root, "contacts").shape,
        TypeDef::Map { .. }
    ));
}

#[test]
fn std_envelope_round_trips_through_cdr() {
    let mut contacts = HashMap::new();
    contacts.insert(
        "127.0.0.1:3838".parse::<SocketAddr>().unwrap(),
        UNIX_EPOCH + Duration::new(123, 456),
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

    let encoded = <StdEnvelope as Message>::Codec::serialize_to_zbuf(&original);
    let decoded = <StdEnvelope as Message>::Codec::deserialize(&encoded.contiguous())
        .expect("wire codec should decode std envelope");

    assert_eq!(decoded, original);
}
