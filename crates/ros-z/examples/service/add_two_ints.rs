use ros_z::{
    Message, ServiceTypeInfo,
    dynamic::{FieldType, MessageSchema},
    msg::Service,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AddTwoIntsRequest {
    pub a: i64,
    pub b: i64,
}

impl Message for AddTwoIntsRequest {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "demo_nodes::AddTwoIntsRequest"
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::entity::SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("demo_nodes::AddTwoIntsRequest")
            .field("a", FieldType::Int64)
            .field("b", FieldType::Int64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsRequest>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AddTwoIntsResponse {
    pub sum: i64,
}

impl Message for AddTwoIntsResponse {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "demo_nodes::AddTwoIntsResponse"
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::entity::SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("demo_nodes::AddTwoIntsResponse")
            .field("sum", FieldType::Int64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsResponse>;
}

pub struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        ros_z::entity::TypeInfo::new("demo_nodes::AddTwoInts", None)
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}
