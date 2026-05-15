use ros_z::{
    Message, SchemaHash,
    schema::{MessageSchema, SchemaBuilder},
};
use ros_z_schema::{SchemaError, TypeDef, TypeName};
use serde::{Deserialize, Serialize};

#[test]
fn type_info_exposes_schema_hash_as_the_public_hash_accessor() {
    let hash = MockMessage::schema_hash();
    let expected_hash = ros_z_schema::compute_hash(&MockMessage::schema())
        .expect("static test schema hash should compute");

    assert_eq!(hash, expected_hash);
    assert!(hash.to_hash_string().starts_with("RZHS02_"));
    assert_eq!(
        SchemaHash::from_hash_string(&hash.to_hash_string()).expect("hash string should parse"),
        hash
    );
}

#[test]
fn type_info_uses_schema_hash() {
    let info = MockMessage::type_info();
    assert_eq!(info.hash, MockMessage::schema_hash());
}

#[test]
fn schema_hash_zero_round_trips() {
    // SchemaHash::zero() should create a valid zero hash
    let zero_hash = SchemaHash::zero();

    assert_eq!(zero_hash.0, [0u8; 32]);
    assert_eq!(
        zero_hash.to_hash_string(),
        "RZHS02_0000000000000000000000000000000000000000000000000000000000000000"
    );

    let parsed_zero = SchemaHash::from_hash_string(
        "RZHS02_0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    assert_eq!(zero_hash, parsed_zero);
}

// Mock message type for testing
#[derive(Debug, Serialize, Deserialize)]
struct MockMessage;

impl Message for MockMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "mock::StaticMessage".to_string()
    }
}

impl MessageSchema for MockMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<String>("name")?;
            fields.field::<String>("hash")?;
            Ok(())
        })
    }
}

#[test]
fn schema_hash_defaults_to_the_message_schema_hash() {
    #[derive(Serialize, Deserialize)]
    struct SimpleMessage;

    impl Message for SimpleMessage {
        type Codec = ros_z::SerdeCdrCodec<Self>;

        fn type_name() -> String {
            "simple::Message".to_string()
        }
    }

    impl MessageSchema for SimpleMessage {
        fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
            builder.define_message_struct::<Self>(|_| Ok(()))
        }
    }

    let expected_hash = SchemaHash(
        ros_z_schema::compute_hash(&SimpleMessage::schema())
            .expect("static test schema hash should compute")
            .0,
    );

    assert_eq!(SimpleMessage::schema_hash(), expected_hash);
}

#[derive(Debug, Serialize, Deserialize)]
struct InvalidStaticSchemaMessage;

impl Message for InvalidStaticSchemaMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "invalid::StaticSchemaMessage".to_string()
    }
}

impl MessageSchema for InvalidStaticSchemaMessage {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        TypeName::new("").map(TypeDef::Named)
    }
}

#[test]
#[should_panic(expected = "message schema should be static and valid")]
fn schema_panics_when_static_message_schema_is_invalid() {
    let _ = InvalidStaticSchemaMessage::schema();
}

#[test]
fn schema_type_info_uses_rzhs_hash_strings() {
    let mut builder = SchemaBuilder::new();
    let name = TypeName::new("std_msgs::String").unwrap();
    let root = builder
        .define_struct(name, |fields| {
            fields.field::<String>("data")?;
            Ok(())
        })
        .unwrap();
    let schema = builder.finish(root).unwrap();

    let hash = SchemaHash(ros_z_schema::compute_hash(&schema).unwrap().0);
    assert!(hash.to_hash_string().starts_with("RZHS02_"));
}

use ros_z::type_info::ServiceTypeInfo;

#[test]
fn type_info_module_exports_runtime_traits() {
    fn assert_service<T: ServiceTypeInfo>() {}

    struct ServiceMarker;
    impl ServiceTypeInfo for ServiceMarker {
        fn service_type_info() -> ros_z::TypeInfo {
            ros_z::TypeInfo::new("test::Service", ros_z::SchemaHash::zero())
        }
    }

    assert_service::<ServiceMarker>();
}
