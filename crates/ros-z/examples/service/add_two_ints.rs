use ros_z::{Message, ServiceTypeInfo, entity::TypeInfo, message::Service};
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
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "demo_nodes::AddTwoInts",
            AddTwoIntsRequest::type_name(),
            AddTwoIntsResponse::type_name(),
        )
        .expect("demo service descriptor should be static and valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("demo service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}
