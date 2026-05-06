use std::sync::Arc;

use ros_z::{
    Message, ServiceTypeInfo,
    dynamic::{FieldSchema, PrimitiveType, Schema, TypeShape},
    msg::Service,
};
use ros_z_schema::TypeName;
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

    fn schema() -> Schema {
        Arc::new(TypeShape::Struct {
            name: TypeName::new("demo_nodes::AddTwoIntsRequest").unwrap(),
            fields: vec![
                FieldSchema::new("a", Arc::new(TypeShape::Primitive(PrimitiveType::I64))),
                FieldSchema::new("b", Arc::new(TypeShape::Primitive(PrimitiveType::I64))),
            ],
        })
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

    fn schema() -> Schema {
        Arc::new(TypeShape::Struct {
            name: TypeName::new("demo_nodes::AddTwoIntsResponse").unwrap(),
            fields: vec![FieldSchema::new(
                "sum",
                Arc::new(TypeShape::Primitive(PrimitiveType::I64)),
            )],
        })
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
