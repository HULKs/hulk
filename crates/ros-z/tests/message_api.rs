use ros_z::msg::{SerdeCdrCodec, WireDecoder, WireEncoder, WireMessage};
use ros_z::schema::{MessageSchema, SchemaBuilder};
use ros_z::{Message, MessageCodec};
use ros_z_schema::{FieldDef, SchemaError, TypeDef, TypeDefinition, TypeName};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiSmokeMessage {
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManualRecursiveNode {
    name: String,
    children: Vec<ManualRecursiveNode>,
}

impl MessageSchema for ManualRecursiveNode {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new(Self::type_name())?;
        builder.define_struct(name, |builder| {
            Ok(vec![
                FieldDef::new("name", String::build_schema(builder)?),
                FieldDef::new(
                    "children",
                    Vec::<ManualRecursiveNode>::build_schema(builder)?,
                ),
            ])
        })
    }
}

impl Message for ManualRecursiveNode {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test::ManualRecursiveNode".to_string()
    }
}

impl Message for ApiSmokeMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "ros_z_tests::ApiSmokeMessage".to_string()
    }
}

impl MessageSchema for ApiSmokeMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new(Self::type_name())?;
        builder.define_struct(name, |builder| {
            Ok(vec![FieldDef::new("value", u32::build_schema(builder)?)])
        })
    }
}

impl WireMessage for ApiSmokeMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;
}

#[test]
fn serde_cdr_codec_roundtrips_message() {
    let original = ApiSmokeMessage { value: 42 };
    let encoded = <ApiSmokeMessage as Message>::Codec::encode(&original).unwrap();
    let decoded =
        <ApiSmokeMessage as Message>::Codec::decode(&encoded.payload.contiguous()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn handwritten_message_constructs_static_pubsub_builders_without_legacy_type_info() {
    let _publisher = ros_z::node::Node::publisher::<ApiSmokeMessage>;
    let _subscriber = ros_z::node::Node::subscriber::<ApiSmokeMessage>;
}

#[test]
fn wire_message_uses_codec_vocabulary() {
    let original = ApiSmokeMessage { value: 7 };
    let encoded = <ApiSmokeMessage as WireMessage>::Codec::serialize_to_zbuf(&original);
    let decoded = <ApiSmokeMessage as WireMessage>::Codec::deserialize(&encoded.contiguous())
        .expect("wire codec should decode encoded message");

    assert_eq!(decoded, original);
}

#[test]
fn primitive_types_are_messages() {
    assert_eq!(u8::type_name(), "u8");
    assert!(matches!(
        u8::schema().unwrap().root,
        TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::U8)
    ));
}

#[test]
fn option_and_vec_types_are_messages() {
    assert_eq!(Option::<u8>::type_name(), "Option<u8>");
    assert_eq!(Vec::<Option<u8>>::type_name(), "Vec<Option<u8>>");
    assert!(matches!(
        Vec::<Option<u8>>::schema().unwrap().root,
        TypeDef::Sequence { .. }
    ));
}

#[test]
fn fixed_arrays_are_fixed_sequences() {
    assert_eq!(<[u8; 16]>::type_name(), "[u8;16]");
    let schema = <[u8; 16]>::schema().unwrap();
    let TypeDef::Sequence { length, .. } = schema.root else {
        panic!("expected sequence schema");
    };
    assert_eq!(length, ros_z_schema::SequenceLengthDef::Fixed(16));
}

#[test]
fn schema_builder_handles_direct_recursion() {
    let schema = ManualRecursiveNode::schema().unwrap();
    let node = TypeName::new("test::ManualRecursiveNode").unwrap();

    assert_eq!(schema.root, TypeDef::Named(node.clone()));
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&node) else {
        panic!("expected ManualRecursiveNode struct definition");
    };
    assert_eq!(definition.fields[0].name, "name");
    assert_eq!(definition.fields[0].shape, TypeDef::String);
    assert_eq!(definition.fields[1].name, "children");
    let TypeDef::Sequence { element, length } = &definition.fields[1].shape else {
        panic!("expected recursive children sequence");
    };
    assert_eq!(*length, ros_z_schema::SequenceLengthDef::Dynamic);
    assert_eq!(element.as_ref(), &TypeDef::Named(node));
    schema.validate().unwrap();
}

#[test]
fn schema_builder_reports_kind_conflicts() {
    let mut builder = SchemaBuilder::new();
    let name = TypeName::new("test::Conflicting").unwrap();

    builder.define_struct(name.clone(), |_| Ok(vec![])).unwrap();
    let error = builder
        .define_enum(name.clone(), |_| {
            Ok(vec![ros_z_schema::EnumVariantDef::new(
                "Variant",
                ros_z_schema::EnumPayloadDef::Unit,
            )])
        })
        .unwrap_err();

    assert!(matches!(error, SchemaError::DefinitionKindConflict { .. }));
}
