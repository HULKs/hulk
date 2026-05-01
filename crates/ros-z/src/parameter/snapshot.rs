use std::sync::Arc;

use ros_z::{FieldTypeInfo, Message, SerdeCdrCodec, dynamic::MessageSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;

use super::{LayerPath, ParameterKey, ProvenanceMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ParameterTimestamp {
    pub sec: i64,
    pub nanosec: u32,
}

impl Message for ParameterTimestamp {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_parameter::ParameterTimestamp"
    }

    fn schema() -> Arc<MessageSchema> {
        MessageSchema::builder("ros_z_parameter::ParameterTimestamp")
            .field("sec", i64::field_type())
            .field("nanosec", u32::field_type())
            .build()
            .expect("failed to build schema for parameter timestamp")
    }

    fn schema_hash() -> ros_z::SchemaHash {
        ros_z::dynamic::schema_hash(&Self::schema()).expect("parameter timestamp schema must hash")
    }
}

impl ParameterTimestamp {
    pub fn now_from(clock: &crate::time::Clock) -> Self {
        let now = clock.now().as_nanos();
        let sec = now.div_euclid(1_000_000_000);
        let nanosec = now.rem_euclid(1_000_000_000) as u32;
        Self { sec, nanosec }
    }
}

#[derive(Debug, Clone)]
pub struct NodeParametersSnapshot<T> {
    pub node_fqn: String,
    pub parameter_key: ParameterKey,
    pub typed: Arc<T>,
    pub effective: Value,
    pub layers: Vec<LayerPath>,
    pub layer_overlays: Vec<Value>,
    pub provenance: Arc<ProvenanceMap>,
    pub revision: u64,
    pub committed_at: ParameterTimestamp,
}

impl<T> NodeParametersSnapshot<T> {
    pub fn typed(&self) -> &T {
        self.typed.as_ref()
    }

    pub fn effective_source_layer(&self, path: &str) -> Option<LayerPath> {
        self.provenance.get(path).cloned()
    }
}

pub type ParameterSubscription<T> = watch::Receiver<Arc<NodeParametersSnapshot<T>>>;
