use super::{ParameterReference, SourceBindingRequest, StreamId};

#[derive(Debug, Clone)]
pub enum WorkerCommand {
    SetIngestEnabled(bool),
    SetDiscoveryNamespace(String),
    BindStream {
        stream_id: StreamId,
        request: SourceBindingRequest,
    },
    RemoveStream {
        stream_id: StreamId,
    },
    ReadParameter(ParameterReference),
    SetParameter {
        target: ParameterReference,
        value_json: String,
    },
    SetScrubAnchor {
        stream_id: StreamId,
        anchor_nanos: u64,
    },
    Shutdown,
}
