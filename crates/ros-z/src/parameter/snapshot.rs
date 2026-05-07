use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;

use super::{LayerPath, ParameterKey, ProvenanceMap};
use crate::time::Clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ros_z::Message)]
#[message(name = "ros_z_parameter::ParameterTimestamp")]
pub struct ParameterTimestamp {
    pub sec: i64,
    pub nanosec: u32,
}

impl ParameterTimestamp {
    pub fn now_from(clock: &Clock) -> Self {
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
