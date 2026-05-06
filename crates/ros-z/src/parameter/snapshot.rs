use std::sync::Arc;

use ros_z::{Message, SerdeCdrCodec};
use ros_z_schema::TypeName;
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

    fn schema() -> ros_z::dynamic::Schema {
        Arc::new(ros_z::dynamic::TypeShape::Struct {
            name: TypeName::new("ros_z_parameter::ParameterTimestamp").expect("valid type name"),
            fields: vec![
                ros_z::dynamic::RuntimeFieldSchema::new("sec", i64::schema()),
                ros_z::dynamic::RuntimeFieldSchema::new("nanosec", u32::schema()),
            ],
        })
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
