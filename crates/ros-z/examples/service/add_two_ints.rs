use ros_z::{
    Message, ServiceTypeInfo,
    entity::{SchemaHash, TypeInfo},
    message::Service,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Message)]
#[message(name = "demo_nodes::AddTwoIntsRequest")]
pub struct AddTwoIntsRequest {
    pub a: i64,
    pub b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Message)]
#[message(name = "demo_nodes::AddTwoIntsResponse")]
pub struct AddTwoIntsResponse {
    pub sum: i64,
}

pub struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> Result<TypeInfo, ros_z_schema::SchemaError> {
        let descriptor = ros_z_schema::ServiceDef::new(
            "demo_nodes::AddTwoInts",
            "demo_nodes::AddTwoIntsRequest",
            "demo_nodes::AddTwoIntsResponse",
        )?;
        Ok(TypeInfo::new(
            "demo_nodes::AddTwoInts",
            Some(SchemaHash(ros_z_schema::compute_hash(&descriptor).0)),
        ))
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}
