use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    FieldTypeInfo, Message, ServiceTypeInfo,
    dynamic::{EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldType, MessageSchema},
    entity::TypeInfo,
    msg::{SerdeCdrCodec, Service, WireMessage},
};

use crate::parameter::{LayerPath, ParameterKey, snapshot::ParameterTimestamp};

pub type JsonPayload = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum NodeParameterChangeSource {
    #[default]
    LocalWrite = 0,
    RemoteWrite = 1,
    Reload = 2,
}

impl FieldTypeInfo for NodeParameterChangeSource {
    fn field_type() -> FieldType {
        FieldType::Enum(Arc::new(EnumSchema::new(
            "ros_z_parameter::NodeParameterChangeSource",
            vec![
                EnumVariantSchema::new("LocalWrite", EnumPayloadSchema::Unit),
                EnumVariantSchema::new("RemoteWrite", EnumPayloadSchema::Unit),
                EnumVariantSchema::new("Reload", EnumPayloadSchema::Unit),
            ],
        )))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParametersSnapshotRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParametersSnapshotResponse {
    pub success: bool,
    pub message: String,
    pub node_fqn: String,
    pub parameter_key: ParameterKey,
    pub revision: u64,
    pub committed_at: ParameterTimestamp,
    pub layers: Vec<LayerPath>,
    pub value_json: JsonPayload,
    pub layer_overlays_json: Vec<JsonPayload>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParameterValueRequest {
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParameterValueResponse {
    pub success: bool,
    pub message: String,
    pub revision: u64,
    pub path: String,
    pub effective_source_layer: LayerPath,
    pub value_json: JsonPayload,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParameterTypeInfoRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNodeParameterTypeInfoResponse {
    pub success: bool,
    pub message: String,
    pub type_name: String,
    pub schema_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetNodeParameterRequest {
    pub path: String,
    pub value_json: JsonPayload,
    pub target_layer: LayerPath,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetNodeParameterResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeParameterWriteJson {
    pub path: String,
    pub value_json: JsonPayload,
    pub target_layer: LayerPath,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetNodeParametersAtomicallyRequest {
    pub writes: Vec<NodeParameterWriteJson>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetNodeParametersAtomicallyResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResetNodeParameterRequest {
    pub path: String,
    pub target_layer: LayerPath,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResetNodeParameterResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReloadNodeParametersRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReloadNodeParametersResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeParameterChange {
    pub path: String,
    pub effective_source_layer: LayerPath,
    pub old_value_json: JsonPayload,
    pub new_value_json: JsonPayload,
}

impl Message for NodeParameterChange {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_parameter::NodeParameterChange"
    }

    fn schema() -> Arc<MessageSchema> {
        MessageSchema::builder("ros_z_parameter::NodeParameterChange")
            .field("path", String::field_type())
            .field("effective_source_layer", LayerPath::field_type())
            .field("old_value_json", String::field_type())
            .field("new_value_json", String::field_type())
            .build()
            .expect("failed to build schema for parameter change")
    }

    fn schema_hash() -> crate::entity::SchemaHash {
        crate::dynamic::schema_hash(&Self::schema()).expect("parameter change schema must hash")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeParameterEvent {
    pub node_fqn: String,
    pub parameter_key: ParameterKey,
    pub previous_revision: u64,
    pub revision: u64,
    pub source: NodeParameterChangeSource,
    pub changed_paths: Vec<String>,
    pub changes: Vec<NodeParameterChange>,
}

impl Message for NodeParameterEvent {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_parameter::NodeParameterEvent"
    }

    fn schema() -> Arc<MessageSchema> {
        MessageSchema::builder("ros_z_parameter::NodeParameterEvent")
            .field("node_fqn", String::field_type())
            .field("parameter_key", ParameterKey::field_type())
            .field("previous_revision", u64::field_type())
            .field("revision", u64::field_type())
            .field(
                "source",
                <NodeParameterChangeSource as FieldTypeInfo>::field_type(),
            )
            .field("changed_paths", Vec::<String>::field_type())
            .field("changes", Vec::<NodeParameterChange>::field_type())
            .build()
            .expect("failed to build schema for parameter event")
    }

    fn schema_hash() -> crate::entity::SchemaHash {
        crate::dynamic::schema_hash(&Self::schema()).expect("parameter event schema must hash")
    }
}

macro_rules! impl_zmessage {
    ($($ty:ty),* $(,)?) => {
        $(impl WireMessage for $ty {
            type Codec = SerdeCdrCodec<Self>;
        })*
    };
}

impl_zmessage!(
    GetNodeParametersSnapshotRequest,
    GetNodeParametersSnapshotResponse,
    GetNodeParameterValueRequest,
    GetNodeParameterValueResponse,
    GetNodeParameterTypeInfoRequest,
    GetNodeParameterTypeInfoResponse,
    SetNodeParameterRequest,
    SetNodeParameterResponse,
    NodeParameterWriteJson,
    SetNodeParametersAtomicallyRequest,
    SetNodeParametersAtomicallyResponse,
    ResetNodeParameterRequest,
    ResetNodeParameterResponse,
    ReloadNodeParametersRequest,
    ReloadNodeParametersResponse,
    NodeParameterChange,
    NodeParameterEvent,
);

macro_rules! impl_service {
    ($srv:ident, $req:ty, $res:ty, $name:literal) => {
        pub struct $srv;

        impl Service for $srv {
            type Request = $req;
            type Response = $res;
        }

        impl ServiceTypeInfo for $srv {
            fn service_type_info() -> TypeInfo {
                TypeInfo::new($name, None)
            }
        }
    };
}

impl_service!(
    GetNodeParametersSnapshotSrv,
    GetNodeParametersSnapshotRequest,
    GetNodeParametersSnapshotResponse,
    "ros_z_parameter::GetNodeParametersSnapshot"
);
impl_service!(
    GetNodeParameterValueSrv,
    GetNodeParameterValueRequest,
    GetNodeParameterValueResponse,
    "ros_z_parameter::GetNodeParameterValue"
);
impl_service!(
    GetNodeParameterTypeInfoSrv,
    GetNodeParameterTypeInfoRequest,
    GetNodeParameterTypeInfoResponse,
    "ros_z_parameter::GetNodeParameterTypeInfo"
);
impl_service!(
    SetNodeParameterSrv,
    SetNodeParameterRequest,
    SetNodeParameterResponse,
    "ros_z_parameter::SetNodeParameter"
);
impl_service!(
    SetNodeParametersAtomicallySrv,
    SetNodeParametersAtomicallyRequest,
    SetNodeParametersAtomicallyResponse,
    "ros_z_parameter::SetNodeParametersAtomically"
);
impl_service!(
    ResetNodeParameterSrv,
    ResetNodeParameterRequest,
    ResetNodeParameterResponse,
    "ros_z_parameter::ResetNodeParameter"
);
impl_service!(
    ReloadNodeParametersSrv,
    ReloadNodeParametersRequest,
    ReloadNodeParametersResponse,
    "ros_z_parameter::ReloadNodeParameters"
);

#[cfg(test)]
mod tests {
    use super::{
        GetNodeParameterTypeInfoSrv, GetNodeParameterValueSrv, GetNodeParametersSnapshotSrv,
        ReloadNodeParametersSrv, ResetNodeParameterSrv, SetNodeParameterSrv,
        SetNodeParametersAtomicallySrv,
    };
    use crate::ServiceTypeInfo;

    #[test]
    fn parameter_service_type_info_uses_native_names() {
        assert_eq!(
            GetNodeParametersSnapshotSrv::service_type_info().name,
            "ros_z_parameter::GetNodeParametersSnapshot"
        );
        assert_eq!(
            GetNodeParameterValueSrv::service_type_info().name,
            "ros_z_parameter::GetNodeParameterValue"
        );
        assert_eq!(
            GetNodeParameterTypeInfoSrv::service_type_info().name,
            "ros_z_parameter::GetNodeParameterTypeInfo"
        );
        assert_eq!(
            SetNodeParameterSrv::service_type_info().name,
            "ros_z_parameter::SetNodeParameter"
        );
        assert_eq!(
            SetNodeParametersAtomicallySrv::service_type_info().name,
            "ros_z_parameter::SetNodeParametersAtomically"
        );
        assert_eq!(
            ResetNodeParameterSrv::service_type_info().name,
            "ros_z_parameter::ResetNodeParameter"
        );
        assert_eq!(
            ReloadNodeParametersSrv::service_type_info().name,
            "ros_z_parameter::ReloadNodeParameters"
        );
    }
}
