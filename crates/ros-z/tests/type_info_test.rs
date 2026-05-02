use ros_z::{Message, SchemaHash};
use ros_z_schema::TypeName;
use serde::{Deserialize, Serialize};

#[test]
fn schema_hash_is_the_public_hash_type() {
    let zero_hash = SchemaHash::zero();

    assert_eq!(zero_hash.0, [0u8; 32]);
    assert_eq!(
        zero_hash.to_hash_string(),
        "RZHS01_0000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn type_info_exposes_schema_hash_as_the_public_hash_accessor() {
    let hash = MockMessage::schema_hash();
    assert_eq!(
        hash.to_hash_string(),
        "RZHS01_1111111111111111111111111111111111111111111111111111111111111111"
    );
}

#[test]
fn type_info_uses_schema_hash() {
    let info = MockMessage::type_info();
    assert_eq!(info.hash, Some(MockMessage::schema_hash()));
}

#[test]
fn schema_hash_zero_round_trips() {
    // SchemaHash::zero() should create a valid zero hash
    let zero_hash = SchemaHash::zero();

    assert_eq!(zero_hash.0, [0u8; 32]);
    assert_eq!(
        zero_hash.to_hash_string(),
        "RZHS01_0000000000000000000000000000000000000000000000000000000000000000"
    );

    let parsed_zero = SchemaHash::from_hash_string(
        "RZHS01_0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    assert_eq!(zero_hash, parsed_zero);
}

// Mock message type for testing
#[derive(Debug, Serialize, Deserialize)]
struct MockMessage;

impl Message for MockMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "mock::StaticMessage"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash::from_hash_string(
            "RZHS01_1111111111111111111111111111111111111111111111111111111111111111",
        )
        .unwrap()
    }

    fn schema() -> ros_z::dynamic::Schema {
        std::sync::Arc::new(ros_z::dynamic::TypeShape::Struct {
            name: TypeName::new("mock::StaticMessage").expect("valid type name"),
            fields: vec![
                ros_z::dynamic::RuntimeFieldSchema::new("name", String::schema()),
                ros_z::dynamic::RuntimeFieldSchema::new("hash", String::schema()),
            ],
        })
    }
}

#[test]
fn test_static_type_info() {
    let static_name = MockMessage::type_name();
    let static_hash = MockMessage::schema_hash();
    let static_info = MockMessage::type_info();

    assert_eq!(static_name, "mock::StaticMessage");

    assert_eq!(
        static_hash.to_hash_string(),
        "RZHS01_1111111111111111111111111111111111111111111111111111111111111111"
    );

    assert_eq!(static_info.name, "mock::StaticMessage");
    assert_eq!(static_info.hash, Some(static_hash));
}

#[test]
fn schema_hash_defaults_to_the_message_schema_hash() {
    #[derive(Serialize, Deserialize)]
    struct SimpleMessage;

    impl Message for SimpleMessage {
        type Codec = ros_z::SerdeCdrCodec<Self>;

        fn type_name() -> &'static str {
            "simple::Message"
        }

        fn schema() -> ros_z::dynamic::Schema {
            std::sync::Arc::new(ros_z::dynamic::TypeShape::Struct {
                name: TypeName::new("simple::Message").expect("valid type name"),
                fields: Vec::new(),
            })
        }
    }

    let expected_hash =
        ros_z::dynamic::schema_tree_hash(SimpleMessage::type_name(), &SimpleMessage::schema())
            .unwrap();

    assert_eq!(SimpleMessage::schema_hash(), expected_hash);
}

#[test]
fn schema_type_info_uses_rzhs_hash_strings() {
    let schema = std::sync::Arc::new(ros_z::dynamic::TypeShape::Struct {
        name: TypeName::new("std_msgs::String").unwrap(),
        fields: vec![ros_z::dynamic::FieldSchema::new(
            "data",
            std::sync::Arc::new(ros_z::dynamic::TypeShape::String),
        )],
    });

    let hash = ros_z::dynamic::schema_tree_hash("std_msgs::String", &schema).unwrap();
    assert!(hash.to_hash_string().starts_with("RZHS01_"));
}

use ros_z::type_info::ServiceTypeInfo;

#[test]
fn type_info_module_exports_runtime_traits() {
    fn assert_service<T: ServiceTypeInfo>() {}

    struct ServiceMarker;
    impl ServiceTypeInfo for ServiceMarker {
        fn service_type_info() -> ros_z::TypeInfo {
            ros_z::TypeInfo::new("test_msgs::ServiceMarker", None)
        }
    }

    assert_service::<ServiceMarker>();
}
