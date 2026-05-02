use ros_z_schema::TypeName;
use serde::{Deserialize, Serialize};

use crate::{
    Message, ServiceTypeInfo,
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

impl Message for NodeParameterChangeSource {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_parameter::NodeParameterChangeSource"
    }

    fn schema() -> crate::dynamic::Schema {
        std::sync::Arc::new(crate::dynamic::TypeShape::Enum {
            name: TypeName::new("ros_z_parameter::NodeParameterChangeSource")
                .expect("valid type name"),
            variants: vec![
                crate::dynamic::RuntimeDynamicEnumVariant::new(
                    "LocalWrite",
                    crate::dynamic::RuntimeDynamicEnumPayload::Unit,
                ),
                crate::dynamic::RuntimeDynamicEnumVariant::new(
                    "RemoteWrite",
                    crate::dynamic::RuntimeDynamicEnumPayload::Unit,
                ),
                crate::dynamic::RuntimeDynamicEnumVariant::new(
                    "Reload",
                    crate::dynamic::RuntimeDynamicEnumPayload::Unit,
                ),
            ],
        })
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

    fn schema() -> crate::dynamic::Schema {
        std::sync::Arc::new(crate::dynamic::TypeShape::Struct {
            name: TypeName::new("ros_z_parameter::NodeParameterChange").expect("valid type name"),
            fields: vec![
                crate::dynamic::RuntimeFieldSchema::new("path", String::schema()),
                crate::dynamic::RuntimeFieldSchema::new("effective_source_layer", String::schema()),
                crate::dynamic::RuntimeFieldSchema::new("old_value_json", String::schema()),
                crate::dynamic::RuntimeFieldSchema::new("new_value_json", String::schema()),
            ],
        })
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

    fn schema() -> crate::dynamic::Schema {
        std::sync::Arc::new(crate::dynamic::TypeShape::Struct {
            name: TypeName::new("ros_z_parameter::NodeParameterEvent").expect("valid type name"),
            fields: vec![
                crate::dynamic::RuntimeFieldSchema::new("node_fqn", String::schema()),
                crate::dynamic::RuntimeFieldSchema::new("parameter_key", String::schema()),
                crate::dynamic::RuntimeFieldSchema::new("previous_revision", u64::schema()),
                crate::dynamic::RuntimeFieldSchema::new("revision", u64::schema()),
                crate::dynamic::RuntimeFieldSchema::new(
                    "source",
                    NodeParameterChangeSource::schema(),
                ),
                crate::dynamic::RuntimeFieldSchema::new("changed_paths", Vec::<String>::schema()),
                crate::dynamic::RuntimeFieldSchema::new(
                    "changes",
                    Vec::<NodeParameterChange>::schema(),
                ),
            ],
        })
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
