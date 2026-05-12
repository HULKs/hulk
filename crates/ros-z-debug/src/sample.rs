use std::sync::Arc;

use ros_z::{EndpointGlobalId, TypeInfo, time::Time};

use crate::TopicSelector;

#[derive(Debug)]
#[non_exhaustive]
pub struct SchemaInfo {
    pub type_name: String,
    pub schema_hash: Option<ros_z::SchemaHash>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct SampleRecord<V> {
    pub value: V,
    pub source_time: Time,
    pub transport_time: Option<Time>,
    pub publication_id: Option<ros_z::pubsub::PublicationId>,
    pub source_global_id: Option<EndpointGlobalId>,
    pub requested_topic: TopicSelector,
    pub resolved_topic: String,
    pub namespace_version: u64,
    pub type_info: Option<TypeInfo>,
    pub schema: Option<Arc<SchemaInfo>>,
}
