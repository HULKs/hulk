use ros_z::{
    Message, SchemaHash,
    schema::{MessageSchema, SchemaBuilder},
};
use ros_z_schema::{SchemaError, TypeDef, TypeName};
use serde::{Deserialize, Serialize};

#[test]
fn type_info_exposes_schema_hash_as_the_public_hash_accessor() {
    let hash = MockMessage::schema_hash().unwrap();
    assert_eq!(
        hash.to_hash_string(),
        "RZHS02_1111111111111111111111111111111111111111111111111111111111111111"
    );
}

#[test]
fn type_info_uses_schema_hash() {
    let info = MockMessage::type_info().unwrap();
    assert_eq!(info.hash, Some(MockMessage::schema_hash().unwrap()));
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

    fn schema_hash() -> Result<SchemaHash, SchemaError> {
        Ok(SchemaHash::from_hash_string(
            "RZHS02_1111111111111111111111111111111111111111111111111111111111111111",
        )
        .unwrap())
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

    let expected_hash = SchemaHash(ros_z_schema::compute_hash(&SimpleMessage::schema().unwrap()).0);

    assert_eq!(SimpleMessage::schema_hash().unwrap(), expected_hash);
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

    let hash = SchemaHash(ros_z_schema::compute_hash(&schema).0);
    assert!(hash.to_hash_string().starts_with("RZHS02_"));
}

use ros_z::type_info::ServiceTypeInfo;

#[test]
fn type_info_module_exports_runtime_traits() {
    fn assert_service<T: ServiceTypeInfo>() {}

    struct ServiceMarker;
    impl ServiceTypeInfo for ServiceMarker {
        fn service_type_info() -> Result<ros_z::TypeInfo, ros_z_schema::SchemaError> {
            Ok(ros_z::TypeInfo::new("test_msgs::ServiceMarker", None))
        }
    }

    assert_service::<ServiceMarker>();
}
