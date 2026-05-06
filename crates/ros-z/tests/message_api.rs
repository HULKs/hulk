use ros_z::msg::{WireDecoder, WireEncoder, WireMessage};
use ros_z::{Message, MessageCodec};
use ros_z_schema::TypeName;
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiSmokeMessage {
    value: u32,
}

impl Message for ApiSmokeMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_tests::ApiSmokeMessage"
    }

    fn schema() -> ros_z::dynamic::Schema {
        std::sync::Arc::new(ros_z::dynamic::TypeShape::Struct {
            name: TypeName::new(Self::type_name()).expect("valid type name"),
            fields: vec![ros_z::dynamic::RuntimeFieldSchema::new(
                "value",
                u32::schema(),
            )],
        })
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::dynamic::schema_tree_hash(Self::type_name(), &Self::schema())
            .expect("schema should hash")
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
        u8::schema().as_ref(),
        ros_z::dynamic::TypeShape::Primitive(ros_z::dynamic::PrimitiveType::U8)
    ));
}

#[test]
fn option_and_vec_types_are_messages() {
    assert_eq!(Option::<u8>::type_name(), "Option<u8>");
    assert_eq!(Vec::<Option<u8>>::type_name(), "Vec<Option<u8>>");
    assert!(matches!(
        Vec::<Option<u8>>::schema().as_ref(),
        ros_z::dynamic::TypeShape::Sequence { .. }
    ));
}

#[test]
fn fixed_arrays_are_fixed_sequences() {
    assert_eq!(<[u8; 16]>::type_name(), "[u8;16]");
    let schema = <[u8; 16]>::schema();
    let ros_z::dynamic::TypeShape::Sequence { length, .. } = schema.as_ref() else {
        panic!("expected sequence schema");
    };
    assert_eq!(*length, ros_z::dynamic::SequenceLength::Fixed(16));
}
