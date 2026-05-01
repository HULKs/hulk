use ros_z::msg::{WireDecoder, WireEncoder, WireMessage};
use ros_z::{Message, MessageCodec};
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

    fn schema() -> std::sync::Arc<ros_z::dynamic::MessageSchema> {
        ros_z::dynamic::MessageSchema::builder(Self::type_name())
            .field("value", ros_z::dynamic::FieldType::Uint32)
            .build()
            .expect("schema should build")
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::dynamic::schema_hash(&Self::schema()).expect("schema should hash")
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
