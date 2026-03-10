use hulkz_stream::PlaneKind;

pub type StreamId = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterReference {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBindingRequest {
    pub namespace: String,
    pub plane: PlaneKind,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceBindingInfo {
    pub namespace: String,
    pub path_expression: String,
}
