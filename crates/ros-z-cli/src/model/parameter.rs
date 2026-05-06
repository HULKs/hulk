use color_eyre::eyre::{Result, eyre};
use ros_z::parameter::{
    GetNodeParameterValueResponse, GetNodeParametersSnapshotResponse, NodeParameterChange,
    NodeParameterChangeSource, NodeParameterEvent, ParameterTimestamp,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ParameterSnapshotView {
    pub node: String,
    pub parameter_key: String,
    pub revision: u64,
    pub committed_at: ParameterTimestamp,
    pub effective: Value,
    pub layers: Vec<String>,
    pub layer_overlays: Vec<Value>,
}

impl ParameterSnapshotView {
    pub fn from_response(response: GetNodeParametersSnapshotResponse) -> Result<Self> {
        Ok(Self {
            node: response.node_fqn,
            parameter_key: response.parameter_key,
            revision: response.revision,
            committed_at: response.committed_at,
            effective: parse_json_field("effective parameter", &response.value_json)?,
            layers: response.layers,
            layer_overlays: response
                .layer_overlays_json
                .iter()
                .enumerate()
                .map(|(index, value)| parse_json_field(&format!("layer overlay {index}"), value))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterValueView {
    pub node: String,
    pub path: String,
    pub revision: u64,
    pub effective_source_layer: String,
    pub value: Value,
}

impl ParameterValueView {
    pub fn from_response(node: String, response: GetNodeParameterValueResponse) -> Result<Self> {
        Ok(Self {
            node,
            path: response.path,
            revision: response.revision,
            effective_source_layer: response.effective_source_layer,
            value: parse_json_field("parameter value", &response.value_json)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterMutationView {
    pub node: String,
    pub operation: String,
    pub path: Option<String>,
    pub target_layer: Option<String>,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
    pub successful: bool,
}

impl ParameterMutationView {
    pub fn new(
        node: String,
        operation: impl Into<String>,
        path: Option<String>,
        target_layer: Option<String>,
        committed_revision: u64,
        changed_paths: Vec<String>,
        successful: bool,
    ) -> Self {
        Self {
            node,
            operation: operation.into(),
            path,
            target_layer,
            committed_revision,
            changed_paths,
            successful,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterWatchEventView {
    pub node: String,
    pub parameter_key: String,
    pub previous_revision: u64,
    pub revision: u64,
    pub source: String,
    pub changed_paths: Vec<String>,
    pub changes: Vec<ParameterWatchChangeView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterWatchChangeView {
    pub path: String,
    pub effective_source_layer: String,
    pub old_value: Value,
    pub new_value: Value,
}

impl ParameterWatchEventView {
    pub fn from_event(event: NodeParameterEvent) -> Result<Self> {
        Ok(Self {
            node: event.node_fqn,
            parameter_key: event.parameter_key,
            previous_revision: event.previous_revision,
            revision: event.revision,
            source: change_source_name(event.source).to_string(),
            changed_paths: event.changed_paths,
            changes: event
                .changes
                .into_iter()
                .map(ParameterWatchChangeView::from_change)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl ParameterWatchChangeView {
    fn from_change(change: NodeParameterChange) -> Result<Self> {
        Ok(Self {
            path: change.path,
            effective_source_layer: change.effective_source_layer,
            old_value: parse_json_field("old parameter value", &change.old_value_json)?,
            new_value: parse_json_field("new parameter value", &change.new_value_json)?,
        })
    }
}

fn parse_json_field(label: &str, value: &str) -> Result<Value> {
    serde_json::from_str(value).map_err(|err| eyre!("failed to parse {label}: {err}"))
}

fn change_source_name(source: NodeParameterChangeSource) -> &'static str {
    match source {
        NodeParameterChangeSource::LocalWrite => "local_write",
        NodeParameterChangeSource::RemoteWrite => "remote_write",
        NodeParameterChangeSource::Reload => "reload",
    }
}
