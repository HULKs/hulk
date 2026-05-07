use serde::{Deserialize, Serialize};

use crate::{ServiceTypeInfo, entity::TypeInfo, message::Service};
use ros_z_schema::{SchemaError, ServiceDef};

use crate::parameter::{LayerPath, ParameterKey, snapshot::ParameterTimestamp};

pub type JsonPayload = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ros_z::Message)]
#[message(name = "ros_z_parameter::NodeParameterChangeSource")]
#[repr(u8)]
pub enum NodeParameterChangeSource {
    #[default]
    LocalWrite = 0,
    RemoteWrite = 1,
    Reload = 2,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParametersSnapshotRequest")]
pub struct GetNodeParametersSnapshotRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParametersSnapshotResponse")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParameterValueRequest")]
pub struct GetNodeParameterValueRequest {
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParameterValueResponse")]
pub struct GetNodeParameterValueResponse {
    pub success: bool,
    pub message: String,
    pub revision: u64,
    pub path: String,
    pub effective_source_layer: LayerPath,
    pub value_json: JsonPayload,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParameterTypeInfoRequest")]
pub struct GetNodeParameterTypeInfoRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::GetNodeParameterTypeInfoResponse")]
pub struct GetNodeParameterTypeInfoResponse {
    pub success: bool,
    pub message: String,
    pub type_name: String,
    pub schema_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::SetNodeParameterRequest")]
pub struct SetNodeParameterRequest {
    pub path: String,
    pub value_json: JsonPayload,
    pub target_layer: LayerPath,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::SetNodeParameterResponse")]
pub struct SetNodeParameterResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::NodeParameterWriteJson")]
pub struct NodeParameterWriteJson {
    pub path: String,
    pub value_json: JsonPayload,
    pub target_layer: LayerPath,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::SetNodeParametersAtomicallyRequest")]
pub struct SetNodeParametersAtomicallyRequest {
    pub writes: Vec<NodeParameterWriteJson>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::SetNodeParametersAtomicallyResponse")]
pub struct SetNodeParametersAtomicallyResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::ResetNodeParameterRequest")]
pub struct ResetNodeParameterRequest {
    pub path: String,
    pub target_layer: LayerPath,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::ResetNodeParameterResponse")]
pub struct ResetNodeParameterResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::ReloadNodeParametersRequest")]
pub struct ReloadNodeParametersRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::ReloadNodeParametersResponse")]
pub struct ReloadNodeParametersResponse {
    pub success: bool,
    pub message: String,
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::NodeParameterChange")]
pub struct NodeParameterChange {
    pub path: String,
    pub effective_source_layer: LayerPath,
    pub old_value_json: JsonPayload,
    pub new_value_json: JsonPayload,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_parameter::NodeParameterEvent")]
pub struct NodeParameterEvent {
    pub node_fqn: String,
    pub parameter_key: ParameterKey,
    pub previous_revision: u64,
    pub revision: u64,
    pub source: NodeParameterChangeSource,
    pub changed_paths: Vec<String>,
    pub changes: Vec<NodeParameterChange>,
}

macro_rules! impl_service {
    ($srv:ident, $req:ty, $res:ty, $name:literal) => {
        pub struct $srv;

        impl Service for $srv {
            type Request = $req;
            type Response = $res;
        }

        impl ServiceTypeInfo for $srv {
            fn service_type_info() -> Result<TypeInfo, SchemaError> {
                let descriptor = ServiceDef::new(
                    $name,
                    <$req as crate::Message>::type_name(),
                    <$res as crate::Message>::type_name(),
                )?;
                Ok(TypeInfo::with_hash(
                    descriptor.type_name.as_str(),
                    ros_z_schema::compute_hash(&descriptor),
                ))
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
            GetNodeParametersSnapshotSrv::service_type_info()
                .unwrap()
                .name,
            "ros_z_parameter::GetNodeParametersSnapshot"
        );
        assert_eq!(
            GetNodeParameterValueSrv::service_type_info().unwrap().name,
            "ros_z_parameter::GetNodeParameterValue"
        );
        assert_eq!(
            GetNodeParameterTypeInfoSrv::service_type_info()
                .unwrap()
                .name,
            "ros_z_parameter::GetNodeParameterTypeInfo"
        );
        assert_eq!(
            SetNodeParameterSrv::service_type_info().unwrap().name,
            "ros_z_parameter::SetNodeParameter"
        );
        assert_eq!(
            SetNodeParametersAtomicallySrv::service_type_info()
                .unwrap()
                .name,
            "ros_z_parameter::SetNodeParametersAtomically"
        );
        assert_eq!(
            ResetNodeParameterSrv::service_type_info().unwrap().name,
            "ros_z_parameter::ResetNodeParameter"
        );
        assert_eq!(
            ReloadNodeParametersSrv::service_type_info().unwrap().name,
            "ros_z_parameter::ReloadNodeParameters"
        );
    }
}
