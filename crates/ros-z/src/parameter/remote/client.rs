use std::{num::NonZeroUsize, sync::Arc};

use crate::{
    ServiceTypeInfo,
    msg::{Service, WireMessage},
    node::Node,
    pubsub::Subscriber,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    service::ServiceClient,
};

use crate::parameter::{LayerPath, ParameterError, Result};

use super::types::{
    GetNodeParameterTypeInfoRequest, GetNodeParameterTypeInfoResponse, GetNodeParameterTypeInfoSrv,
    GetNodeParameterValueRequest, GetNodeParameterValueResponse, GetNodeParameterValueSrv,
    GetNodeParametersSnapshotRequest, GetNodeParametersSnapshotResponse,
    GetNodeParametersSnapshotSrv, NodeParameterEvent, NodeParameterWriteJson,
    ReloadNodeParametersRequest, ReloadNodeParametersResponse, ReloadNodeParametersSrv,
    ResetNodeParameterRequest, ResetNodeParameterResponse, ResetNodeParameterSrv,
    SetNodeParameterRequest, SetNodeParameterResponse, SetNodeParameterSrv,
    SetNodeParametersAtomicallyRequest, SetNodeParametersAtomicallyResponse,
    SetNodeParametersAtomicallySrv,
};

const PARAMETER_EVENT_HISTORY_DEPTH: usize = 64;

#[derive(Debug, Clone)]
pub struct RemoteParameterClient {
    node: Arc<Node>,
    target_node_fqn: String,
}

impl RemoteParameterClient {
    pub fn new(node: Arc<Node>, target_node_fqn: impl Into<String>) -> Result<Self> {
        let target_node_fqn = target_node_fqn.into();
        if !target_node_fqn.starts_with('/') {
            return Err(ParameterError::RemoteError {
                message: format!(
                    "remote parameter target must be an absolute node FQN, got '{target_node_fqn}'"
                ),
            });
        }

        Ok(Self {
            node,
            target_node_fqn,
        })
    }

    pub fn target_node_fqn(&self) -> &str {
        &self.target_node_fqn
    }

    pub fn service_name(&self, suffix: &str) -> String {
        format!("{}/parameter/{suffix}", self.target_node_fqn)
    }

    pub fn events_topic(&self) -> String {
        format!("{}/parameter/events", self.target_node_fqn)
    }

    pub async fn get_snapshot(&self) -> Result<GetNodeParametersSnapshotResponse> {
        self.call_service::<GetNodeParametersSnapshotSrv>(
            &self.service_name("get_snapshot"),
            &GetNodeParametersSnapshotRequest {},
        )
        .await
    }

    pub async fn get_value(
        &self,
        path: impl Into<String>,
    ) -> Result<GetNodeParameterValueResponse> {
        self.call_service::<GetNodeParameterValueSrv>(
            &self.service_name("get_value"),
            &GetNodeParameterValueRequest { path: path.into() },
        )
        .await
    }

    pub async fn get_type_info(&self) -> Result<GetNodeParameterTypeInfoResponse> {
        self.call_service::<GetNodeParameterTypeInfoSrv>(
            &self.service_name("get_type_info"),
            &GetNodeParameterTypeInfoRequest {},
        )
        .await
    }

    pub async fn set_json(
        &self,
        path: impl Into<String>,
        value: &serde_json::Value,
        target_layer: impl Into<LayerPath>,
        expected_revision: Option<u64>,
    ) -> Result<SetNodeParameterResponse> {
        self.call_service::<SetNodeParameterSrv>(
            &self.service_name("set"),
            &SetNodeParameterRequest {
                path: path.into(),
                value_json: serialize_json(value)?,
                target_layer: target_layer.into(),
                expected_revision,
            },
        )
        .await
    }

    pub async fn set_json_atomically(
        &self,
        writes: Vec<NodeParameterWriteJson>,
        expected_revision: Option<u64>,
    ) -> Result<SetNodeParametersAtomicallyResponse> {
        self.call_service::<SetNodeParametersAtomicallySrv>(
            &self.service_name("set_atomic"),
            &SetNodeParametersAtomicallyRequest {
                writes,
                expected_revision,
            },
        )
        .await
    }

    pub async fn reset(
        &self,
        path: impl Into<String>,
        target_layer: impl Into<LayerPath>,
        expected_revision: Option<u64>,
    ) -> Result<ResetNodeParameterResponse> {
        self.call_service::<ResetNodeParameterSrv>(
            &self.service_name("reset"),
            &ResetNodeParameterRequest {
                path: path.into(),
                target_layer: target_layer.into(),
                expected_revision,
            },
        )
        .await
    }

    pub async fn reload(&self) -> Result<ReloadNodeParametersResponse> {
        self.call_service::<ReloadNodeParametersSrv>(
            &self.service_name("reload"),
            &ReloadNodeParametersRequest {},
        )
        .await
    }

    pub async fn subscribe_events(&self) -> Result<Subscriber<NodeParameterEvent>> {
        self.node
            .subscriber::<NodeParameterEvent>(&self.events_topic())
            .qos(QosProfile {
                reliability: QosReliability::Reliable,
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(
                    NonZeroUsize::new(PARAMETER_EVENT_HISTORY_DEPTH).expect("non-zero"),
                ),
                ..Default::default()
            })
            .build()
            .await
            .map_err(map_remote_err)
    }

    async fn call_service<S>(&self, service_name: &str, request: &S::Request) -> Result<S::Response>
    where
        S: Service + ServiceTypeInfo,
        S::Request: WireMessage,
        S::Response: WireMessage,
        for<'a> <S::Response as WireMessage>::Codec:
            crate::msg::WireDecoder<Output = S::Response, Input<'a> = &'a [u8]>,
    {
        let client = self.build_client::<S>(service_name).await?;
        client.call_async(request).await.map_err(map_remote_err)
    }

    async fn build_client<S>(&self, service_name: &str) -> Result<ServiceClient<S>>
    where
        S: Service + ServiceTypeInfo,
    {
        self.node
            .create_service_client::<S>(service_name)
            .build()
            .await
            .map_err(map_remote_err)
    }
}

fn serialize_json(value: &serde_json::Value) -> Result<String> {
    serde_json::to_string(value).map_err(|err| ParameterError::RemoteError {
        message: format!("failed to serialize JSON payload: {err}"),
    })
}

fn map_remote_err<E: std::fmt::Display>(err: E) -> ParameterError {
    ParameterError::RemoteError {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::context::ContextBuilder;

    use super::RemoteParameterClient;

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_non_absolute_target_fqn() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("build context");
        let node = Arc::new(
            context
                .create_node("tester")
                .build()
                .await
                .expect("build node"),
        );
        let err = RemoteParameterClient::new(node, "vision/ball_detector")
            .expect_err("must reject relative target");
        assert!(err.to_string().contains("absolute node FQN"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn builds_absolute_service_and_event_names() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("build context");
        let node = Arc::new(
            context
                .create_node("tester")
                .build()
                .await
                .expect("build node"),
        );
        let client =
            RemoteParameterClient::new(node, "/vision/ball_detector").expect("build client");

        assert_eq!(
            client.service_name("get_snapshot"),
            "/vision/ball_detector/parameter/get_snapshot"
        );
        assert_eq!(
            client.service_name("set"),
            "/vision/ball_detector/parameter/set"
        );
        assert_eq!(
            client.events_topic(),
            "/vision/ball_detector/parameter/events"
        );
    }
}
