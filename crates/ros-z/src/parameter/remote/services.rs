use std::sync::Arc;

use serde::{Serialize, de::DeserializeOwned};
use zenoh::Wait;

use crate::{
    Message,
    attachment::Attachment,
    msg::WireMessage,
    node::Node,
    parameter::{NodeParameters, ParameterError, Result},
    pubsub::Publisher,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    service::ServiceServer,
};

use crate::parameter::node_parameter::{CommitOutcome, NodeParametersInner, ParameterJsonWrite};

use super::types::*;

pub struct RemoteParameterServices<T> {
    event_publisher: Publisher<NodeParameterEvent>,
    _get_snapshot: Arc<ServiceServer<GetNodeParametersSnapshotSrv, ()>>,
    _get_value: Arc<ServiceServer<GetNodeParameterValueSrv, ()>>,
    _get_type_info: Arc<ServiceServer<GetNodeParameterTypeInfoSrv, ()>>,
    _set: Arc<ServiceServer<SetNodeParameterSrv, ()>>,
    _set_atomic: Arc<ServiceServer<SetNodeParametersAtomicallySrv, ()>>,
    _reset: Arc<ServiceServer<ResetNodeParameterSrv, ()>>,
    _reload: Arc<ServiceServer<ReloadNodeParametersSrv, ()>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RemoteParameterServices<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    pub async fn register(node: &Node, inner: Arc<NodeParametersInner<T>>) -> Result<Self> {
        let event_publisher = node
            .publisher::<NodeParameterEvent>("~parameter/events")
            .qos(QosProfile {
                reliability: QosReliability::Reliable,
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(std::num::NonZeroUsize::new(64).expect("non-zero")),
                ..Default::default()
            })
            .build()
            .await
            .map_err(|err| ParameterError::RemoteError {
                message: err.to_string(),
            })?;

        let get_snapshot =
            register_server::<GetNodeParametersSnapshotSrv>(node, "~parameter/get_snapshot", {
                let inner = inner.clone();
                move |query| handle_get_snapshot::<T>(&inner, query)
            })
            .await?;

        let get_value =
            register_server::<GetNodeParameterValueSrv>(node, "~parameter/get_value", {
                let inner = inner.clone();
                move |query| handle_get_value::<T>(&inner, query)
            })
            .await?;

        let get_type_info =
            register_server::<GetNodeParameterTypeInfoSrv>(node, "~parameter/get_type_info", {
                let inner = inner.clone();
                move |query| handle_get_type_info::<T>(&inner, query)
            })
            .await?;

        let set = register_server::<SetNodeParameterSrv>(node, "~parameter/set", {
            let inner = inner.clone();
            move |query| handle_set::<T>(&inner, query)
        })
        .await?;

        let set_atomic =
            register_server::<SetNodeParametersAtomicallySrv>(node, "~parameter/set_atomic", {
                let inner = inner.clone();
                move |query| handle_set_atomic::<T>(&inner, query)
            })
            .await?;

        let reset = register_server::<ResetNodeParameterSrv>(node, "~parameter/reset", {
            let inner = inner.clone();
            move |query| handle_reset::<T>(&inner, query)
        })
        .await?;

        let reload = register_server::<ReloadNodeParametersSrv>(node, "~parameter/reload", {
            let inner = inner.clone();
            move |query| handle_reload::<T>(&inner, query)
        })
        .await?;

        Ok(Self {
            event_publisher,
            _get_snapshot: Arc::new(get_snapshot),
            _get_value: Arc::new(get_value),
            _get_type_info: Arc::new(get_type_info),
            _set: Arc::new(set),
            _set_atomic: Arc::new(set_atomic),
            _reset: Arc::new(reset),
            _reload: Arc::new(reload),
            _phantom: std::marker::PhantomData,
        })
    }

    pub async fn publish_event(&self, event: &NodeParameterEvent) -> Result<()> {
        self.event_publisher
            .publish(event)
            .await
            .map_err(|err| ParameterError::RemoteError {
                message: err.to_string(),
            })
    }
}

async fn register_server<S>(
    node: &Node,
    name: &str,
    handler: impl Fn(&zenoh::query::Query) + Send + Sync + 'static,
) -> Result<ServiceServer<S, ()>>
where
    S: crate::msg::Service + crate::ServiceTypeInfo,
{
    node.create_service_server::<S>(name)
        .build_with_callback(move |query| handler(&query))
        .await
        .map_err(|err| ParameterError::RemoteError {
            message: err.to_string(),
        })
}

fn handle_get_snapshot<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let snapshot = parameters.snapshot();
    let response = GetNodeParametersSnapshotResponse {
        success: true,
        message: String::new(),
        node_fqn: snapshot.node_fqn.clone(),
        parameter_key: snapshot.parameter_key.clone(),
        revision: snapshot.revision,
        committed_at: snapshot.committed_at,
        layers: snapshot.layers.clone(),
        value_json: to_json(&snapshot.effective),
        layer_overlays_json: snapshot.layer_overlays.iter().map(to_json).collect(),
    };
    reply(query, &response);
}

fn handle_get_value<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let request = decode_request::<GetNodeParameterValueRequest>(query);
    let response = match request {
        Ok(request) => {
            let snapshot = parameters.snapshot();
            match parameters.get_json(&request.path) {
                Ok(value) => GetNodeParameterValueResponse {
                    success: true,
                    message: String::new(),
                    revision: snapshot.revision,
                    path: request.path.clone(),
                    effective_source_layer: snapshot
                        .effective_source_layer(&request.path)
                        .unwrap_or_default(),
                    value_json: to_json(&value),
                },
                Err(err) => GetNodeParameterValueResponse {
                    success: false,
                    message: err.to_string(),
                    revision: snapshot.revision,
                    path: request.path,
                    effective_source_layer: String::new(),
                    value_json: "null".to_string(),
                },
            }
        }
        Err(message) => GetNodeParameterValueResponse {
            success: false,
            message,
            revision: 0,
            path: String::new(),
            effective_source_layer: String::new(),
            value_json: "null".to_string(),
        },
    };
    reply(query, &response);
}

fn handle_get_type_info<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    reply(
        query,
        &GetNodeParameterTypeInfoResponse {
            success: true,
            message: String::new(),
            type_name: inner.type_name.clone(),
            schema_hash: inner.schema_hash.to_hash_string(),
        },
    );
}

fn handle_set<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let request = decode_request::<SetNodeParameterRequest>(query);
    let response = match request {
        Ok(request) => match serde_json::from_str::<serde_json::Value>(&request.value_json) {
            Ok(value) => match parameters.commit(
                &[ParameterJsonWrite {
                    path: request.path,
                    value,
                    target_layer: request.target_layer,
                }],
                &[],
                request.expected_revision,
                NodeParameterChangeSource::RemoteWrite,
            ) {
                Ok(outcome) => write_response(outcome),
                Err(err) => error_write_response(err),
            },
            Err(err) => error_write_response(ParameterError::RemoteError {
                message: err.to_string(),
            }),
        },
        Err(message) => SetNodeParameterResponse {
            success: false,
            message,
            committed_revision: 0,
            changed_paths: Vec::new(),
        },
    };
    reply(query, &response);
}

fn handle_set_atomic<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let request = decode_request::<SetNodeParametersAtomicallyRequest>(query);
    let response = match request {
        Ok(request) => {
            let mut writes = Vec::with_capacity(request.writes.len());
            let mut parse_error = None;
            for write in request.writes {
                match serde_json::from_str::<serde_json::Value>(&write.value_json) {
                    Ok(value) => writes.push(ParameterJsonWrite {
                        path: write.path,
                        value,
                        target_layer: write.target_layer,
                    }),
                    Err(err) => {
                        parse_error = Some(err.to_string());
                        break;
                    }
                }
            }

            if let Some(message) = parse_error {
                error_write_response(ParameterError::RemoteError { message })
            } else {
                match parameters.commit(
                    &writes,
                    &[],
                    request.expected_revision,
                    NodeParameterChangeSource::RemoteWrite,
                ) {
                    Ok(outcome) => write_response(outcome),
                    Err(err) => error_write_response(err),
                }
            }
        }
        Err(message) => SetNodeParametersAtomicallyResponse {
            success: false,
            message,
            committed_revision: 0,
            changed_paths: Vec::new(),
        },
    };
    reply(query, &response);
}

fn handle_reset<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let request = decode_request::<ResetNodeParameterRequest>(query);
    let response = match request {
        Ok(request) => match parameters.commit(
            &[],
            &[(request.path, request.target_layer)],
            request.expected_revision,
            NodeParameterChangeSource::RemoteWrite,
        ) {
            Ok(outcome) => write_response(outcome),
            Err(err) => error_write_response(err),
        },
        Err(message) => ResetNodeParameterResponse {
            success: false,
            message,
            committed_revision: 0,
            changed_paths: Vec::new(),
        },
    };
    reply(query, &response);
}

fn handle_reload<T>(inner: &Arc<NodeParametersInner<T>>, query: &zenoh::query::Query)
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let parameters = NodeParameters {
        inner: inner.clone(),
    };
    let response = match parameters.reload_with_source(NodeParameterChangeSource::Reload) {
        Ok(outcome) => ReloadNodeParametersResponse {
            success: true,
            message: String::new(),
            committed_revision: outcome.committed_revision,
            changed_paths: outcome.changed_paths,
        },
        Err(err) => ReloadNodeParametersResponse {
            success: false,
            message: err.to_string(),
            committed_revision: 0,
            changed_paths: Vec::new(),
        },
    };
    reply(query, &response);
}

fn write_response<T>(outcome: CommitOutcome) -> T
where
    T: From<(bool, String, u64, Vec<String>)>,
{
    T::from((
        true,
        String::new(),
        outcome.committed_revision,
        outcome.changed_paths,
    ))
}

fn error_write_response<T>(err: ParameterError) -> T
where
    T: From<(bool, String, u64, Vec<String>)>,
{
    T::from((false, err.to_string(), 0, Vec::new()))
}

impl From<(bool, String, u64, Vec<String>)> for SetNodeParameterResponse {
    fn from(value: (bool, String, u64, Vec<String>)) -> Self {
        Self {
            success: value.0,
            message: value.1,
            committed_revision: value.2,
            changed_paths: value.3,
        }
    }
}

impl From<(bool, String, u64, Vec<String>)> for SetNodeParametersAtomicallyResponse {
    fn from(value: (bool, String, u64, Vec<String>)) -> Self {
        Self {
            success: value.0,
            message: value.1,
            committed_revision: value.2,
            changed_paths: value.3,
        }
    }
}

impl From<(bool, String, u64, Vec<String>)> for ResetNodeParameterResponse {
    fn from(value: (bool, String, u64, Vec<String>)) -> Self {
        Self {
            success: value.0,
            message: value.1,
            committed_revision: value.2,
            changed_paths: value.3,
        }
    }
}

fn decode_request<T>(query: &zenoh::query::Query) -> std::result::Result<T, String>
where
    T: WireMessage,
    for<'a> <T as WireMessage>::Codec: crate::msg::WireDecoder<Output = T, Input<'a> = &'a [u8]>,
{
    let payload = query
        .payload()
        .ok_or_else(|| "missing request payload".to_string())?;
    T::deserialize(payload.to_bytes().as_ref()).map_err(|err| err.to_string())
}

fn reply<T>(query: &zenoh::query::Query, response: &T)
where
    T: WireMessage,
{
    let bytes = response.serialize();
    let mut reply = query.reply(query.key_expr().clone(), bytes);
    if let Some(att_bytes) = query.attachment()
        && let Ok(att) = Attachment::try_from(att_bytes)
    {
        reply = reply.attachment(att);
    }
    if let Err(err) = reply.wait() {
        tracing::warn!("[PARAM] Failed to send parameter reply: {err}");
    }
}

fn to_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}
