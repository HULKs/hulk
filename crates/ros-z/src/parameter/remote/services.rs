use std::{marker::PhantomData, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{runtime::Handle, sync::mpsc};
use zenoh::query::Query;

use crate::{
    Message, ServiceTypeInfo,
    attachment::Attachment,
    message::{Service, WireDecoder, WireEncoder},
    node::Node,
    parameter::{ParameterError, Result},
    pubsub::Publisher,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    service::ServiceServer,
};

use crate::parameter::{
    ParameterTimestamp,
    node_parameter::{
        CommitOutcome, ParameterCommand, ParameterDriver, ParameterJsonWrite, ParameterState,
        RemoteParameterCommand,
    },
};

use super::types::*;

pub struct RemoteParameterServices<T> {
    event_publisher: Arc<Publisher<NodeParameterEvent>>,
    _get_snapshot: Arc<ServiceServer<GetNodeParametersSnapshotSrv, ()>>,
    _get_value: Arc<ServiceServer<GetNodeParameterValueSrv, ()>>,
    _get_type_info: Arc<ServiceServer<GetNodeParameterTypeInfoSrv, ()>>,
    _set: Arc<ServiceServer<SetNodeParameterSrv, ()>>,
    _set_atomic: Arc<ServiceServer<SetNodeParametersAtomicallySrv, ()>>,
    _reset: Arc<ServiceServer<ResetNodeParameterSrv, ()>>,
    _reload: Arc<ServiceServer<ReloadNodeParametersSrv, ()>>,
    _phantom: PhantomData<T>,
}

impl<T> RemoteParameterServices<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    pub async fn register(
        node: &Node,
        commands: mpsc::Sender<ParameterCommand<T>>,
        reply_runtime: Handle,
    ) -> Result<Self> {
        let event_publisher = node
            .publisher::<NodeParameterEvent>("~parameter/events")
            .map_err(|err| ParameterError::RemoteError {
                message: err.to_string(),
            })?
            .qos(QosProfile {
                reliability: QosReliability::Reliable,
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(std::num::NonZeroUsize::new(64).expect("non-zero")),
                ..Default::default()
            })
            .build()
            .await
            .map_err(|source| {
                ParameterError::operation("creating parameter event publisher", source)
            })?;

        let get_snapshot =
            register_server::<GetNodeParametersSnapshotSrv>(node, "~parameter/get_snapshot", {
                let commands = commands.clone();
                let reply_runtime = reply_runtime.clone();
                move |query| {
                    enqueue_or_reply_busy(
                        &commands,
                        &reply_runtime,
                        query,
                        |query| RemoteParameterCommand::GetSnapshot { query },
                        busy_snapshot_response,
                    );
                }
            })
            .await?;

        let get_value =
            register_server::<GetNodeParameterValueSrv>(node, "~parameter/get_value", {
                let commands = commands.clone();
                let reply_runtime = reply_runtime.clone();
                move |query| {
                    enqueue_or_reply_busy(
                        &commands,
                        &reply_runtime,
                        query,
                        |query| RemoteParameterCommand::GetValue { query },
                        busy_value_response,
                    );
                }
            })
            .await?;

        let get_type_info =
            register_server::<GetNodeParameterTypeInfoSrv>(node, "~parameter/get_type_info", {
                let commands = commands.clone();
                let reply_runtime = reply_runtime.clone();
                move |query| {
                    enqueue_or_reply_busy(
                        &commands,
                        &reply_runtime,
                        query,
                        |query| RemoteParameterCommand::GetTypeInfo { query },
                        busy_type_info_response,
                    );
                }
            })
            .await?;

        let set = register_server::<SetNodeParameterSrv>(node, "~parameter/set", {
            let commands = commands.clone();
            let reply_runtime = reply_runtime.clone();
            move |query| {
                enqueue_or_reply_busy(
                    &commands,
                    &reply_runtime,
                    query,
                    |query| RemoteParameterCommand::Set { query },
                    busy_write_response::<SetNodeParameterResponse>,
                );
            }
        })
        .await?;

        let set_atomic =
            register_server::<SetNodeParametersAtomicallySrv>(node, "~parameter/set_atomic", {
                let commands = commands.clone();
                let reply_runtime = reply_runtime.clone();
                move |query| {
                    enqueue_or_reply_busy(
                        &commands,
                        &reply_runtime,
                        query,
                        |query| RemoteParameterCommand::SetAtomic { query },
                        busy_write_response::<SetNodeParametersAtomicallyResponse>,
                    );
                }
            })
            .await?;

        let reset = register_server::<ResetNodeParameterSrv>(node, "~parameter/reset", {
            let commands = commands.clone();
            let reply_runtime = reply_runtime.clone();
            move |query| {
                enqueue_or_reply_busy(
                    &commands,
                    &reply_runtime,
                    query,
                    |query| RemoteParameterCommand::Reset { query },
                    busy_write_response::<ResetNodeParameterResponse>,
                );
            }
        })
        .await?;

        let reload = register_server::<ReloadNodeParametersSrv>(node, "~parameter/reload", {
            let commands = commands.clone();
            let reply_runtime = reply_runtime.clone();
            move |query| {
                enqueue_or_reply_busy(
                    &commands,
                    &reply_runtime,
                    query,
                    |query| RemoteParameterCommand::Reload { query },
                    busy_write_response::<ReloadNodeParametersResponse>,
                );
            }
        })
        .await?;

        Ok(Self {
            event_publisher: Arc::new(event_publisher),
            _get_snapshot: Arc::new(get_snapshot),
            _get_value: Arc::new(get_value),
            _get_type_info: Arc::new(get_type_info),
            _set: Arc::new(set),
            _set_atomic: Arc::new(set_atomic),
            _reset: Arc::new(reset),
            _reload: Arc::new(reload),
            _phantom: PhantomData,
        })
    }

    pub fn event_publisher(&self) -> Arc<Publisher<NodeParameterEvent>> {
        self.event_publisher.clone()
    }
}

async fn register_server<S>(
    node: &Node,
    name: &str,
    handler: impl Fn(Query) + Send + Sync + 'static,
) -> Result<ServiceServer<S, ()>>
where
    S: Service + ServiceTypeInfo,
{
    let operation = format!("creating parameter service server '{name}'");
    node.create_service_server::<S>(name)
        .map_err(|err| ParameterError::RemoteError {
            message: err.to_string(),
        })?
        .build_with_callback(handler)
        .await
        .map_err(|source| ParameterError::operation(operation, source))
}

const PARAMETER_ACTOR_BUSY: &str = "parameter actor is unavailable or busy";

fn busy_snapshot_response() -> GetNodeParametersSnapshotResponse {
    GetNodeParametersSnapshotResponse {
        success: false,
        message: PARAMETER_ACTOR_BUSY.to_string(),
        node_fqn: String::new(),
        parameter_key: String::new(),
        revision: 0,
        committed_at: ParameterTimestamp::default(),
        layers: Vec::new(),
        value_json: "null".to_string(),
        layer_overlays_json: Vec::new(),
    }
}

fn busy_value_response() -> GetNodeParameterValueResponse {
    GetNodeParameterValueResponse {
        success: false,
        message: PARAMETER_ACTOR_BUSY.to_string(),
        revision: 0,
        path: String::new(),
        effective_source_layer: String::new(),
        value_json: "null".to_string(),
    }
}

fn busy_type_info_response() -> GetNodeParameterTypeInfoResponse {
    GetNodeParameterTypeInfoResponse {
        success: false,
        message: PARAMETER_ACTOR_BUSY.to_string(),
        type_name: String::new(),
        schema_hash: String::new(),
    }
}

fn busy_write_response<T>() -> T
where
    T: From<(bool, String, u64, Vec<String>)>,
{
    T::from((false, PARAMETER_ACTOR_BUSY.to_string(), 0, Vec::new()))
}

fn enqueue_or_reply_busy<T, R>(
    commands: &mpsc::Sender<ParameterCommand<T>>,
    reply_runtime: &Handle,
    query: Query,
    make_command: impl FnOnce(Query) -> RemoteParameterCommand,
    make_busy_response: impl FnOnce() -> R,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
    R: Message + Send + 'static,
    for<'a> <R as Message>::Codec: WireEncoder<Input<'a> = &'a R>,
{
    match commands.try_send(ParameterCommand::Remote(make_command(query))) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(ParameterCommand::Remote(command)))
        | Err(mpsc::error::TrySendError::Closed(ParameterCommand::Remote(command))) => {
            let query = command.into_query();
            let busy_response = make_busy_response();
            spawn_reply(reply_runtime, query, busy_response);
        }
        Err(mpsc::error::TrySendError::Full(_)) | Err(mpsc::error::TrySendError::Closed(_)) => {
            tracing::warn!("[PARAM] Unexpected local command returned from remote enqueue");
        }
    }
}

pub fn handle_get_snapshot_for_state<T>(
    state: &Arc<ParameterState<T>>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let snapshot = state.snapshot();
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
    spawn_reply(reply_runtime, query, response);
}

pub fn handle_get_value_for_state<T>(
    state: &Arc<ParameterState<T>>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let request = decode_request::<GetNodeParameterValueRequest>(&query);
    let response = match request {
        Ok(request) => {
            let snapshot = state.snapshot();
            match ParameterState::get_json_from_snapshot(&snapshot, &request.path) {
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
    spawn_reply(reply_runtime, query, response);
}

pub fn handle_get_type_info_for_state<T>(
    state: &Arc<ParameterState<T>>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let response = GetNodeParameterTypeInfoResponse {
        success: true,
        message: String::new(),
        type_name: state.type_name().to_string(),
        schema_hash: state.schema_hash().to_hash_string(),
    };
    spawn_reply(reply_runtime, query, response);
}

pub async fn handle_set_for_driver<T>(
    driver: &ParameterDriver<T>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let request = decode_request::<SetNodeParameterRequest>(&query);
    let response = match request {
        Ok(request) => match serde_json::from_str::<serde_json::Value>(&request.value_json) {
            Ok(value) => match driver
                .commit(
                    &[ParameterJsonWrite {
                        path: request.path,
                        value,
                        target_layer: request.target_layer,
                    }],
                    &[],
                    request.expected_revision,
                    NodeParameterChangeSource::RemoteWrite,
                )
                .await
            {
                Ok(outcome) => write_response(outcome),
                Err(err) => error_write_response(err),
            },
            Err(source) => error_write_response(ParameterError::RemotePayloadParseError { source }),
        },
        Err(message) => SetNodeParameterResponse {
            success: false,
            message,
            committed_revision: 0,
            changed_paths: Vec::new(),
        },
    };
    spawn_reply(reply_runtime, query, response);
}

pub async fn handle_set_atomic_for_driver<T>(
    driver: &ParameterDriver<T>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let request = decode_request::<SetNodeParametersAtomicallyRequest>(&query);
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
                    Err(source) => {
                        parse_error = Some(source);
                        break;
                    }
                }
            }

            if let Some(source) = parse_error {
                error_write_response(ParameterError::RemotePayloadParseError { source })
            } else {
                match driver
                    .commit(
                        &writes,
                        &[],
                        request.expected_revision,
                        NodeParameterChangeSource::RemoteWrite,
                    )
                    .await
                {
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
    spawn_reply(reply_runtime, query, response);
}

pub async fn handle_reset_for_driver<T>(
    driver: &ParameterDriver<T>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let request = decode_request::<ResetNodeParameterRequest>(&query);
    let response = match request {
        Ok(request) => match driver
            .commit(
                &[],
                &[(request.path, request.target_layer)],
                request.expected_revision,
                NodeParameterChangeSource::RemoteWrite,
            )
            .await
        {
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
    spawn_reply(reply_runtime, query, response);
}

pub async fn handle_reload_for_driver<T>(
    driver: &ParameterDriver<T>,
    reply_runtime: &Handle,
    query: Query,
) where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let response = match driver
        .reload_with_source(NodeParameterChangeSource::Reload)
        .await
    {
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
    spawn_reply(reply_runtime, query, response);
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

impl From<(bool, String, u64, Vec<String>)> for ReloadNodeParametersResponse {
    fn from(value: (bool, String, u64, Vec<String>)) -> Self {
        Self {
            success: value.0,
            message: value.1,
            committed_revision: value.2,
            changed_paths: value.3,
        }
    }
}

fn decode_request<T>(query: &Query) -> std::result::Result<T, String>
where
    T: Message,
    for<'a> <T as Message>::Codec: WireDecoder<Output = T, Input<'a> = &'a [u8]>,
{
    let payload = query
        .payload()
        .ok_or_else(|| "missing request payload".to_string())?;
    <<T as Message>::Codec as WireDecoder>::deserialize(payload.to_bytes().as_ref())
        .map_err(|err| format!("{err}"))
}

pub(crate) fn reply_remote_command_unavailable(
    reply_runtime: &Handle,
    command: RemoteParameterCommand,
) {
    match command {
        RemoteParameterCommand::GetSnapshot { query } => {
            spawn_reply(reply_runtime, query, busy_snapshot_response());
        }
        RemoteParameterCommand::GetValue { query } => {
            spawn_reply(reply_runtime, query, busy_value_response());
        }
        RemoteParameterCommand::GetTypeInfo { query } => {
            spawn_reply(reply_runtime, query, busy_type_info_response());
        }
        RemoteParameterCommand::Set { query } => {
            spawn_reply(
                reply_runtime,
                query,
                busy_write_response::<SetNodeParameterResponse>(),
            );
        }
        RemoteParameterCommand::SetAtomic { query } => {
            spawn_reply(
                reply_runtime,
                query,
                busy_write_response::<SetNodeParametersAtomicallyResponse>(),
            );
        }
        RemoteParameterCommand::Reset { query } => {
            spawn_reply(
                reply_runtime,
                query,
                busy_write_response::<ResetNodeParameterResponse>(),
            );
        }
        RemoteParameterCommand::Reload { query } => {
            spawn_reply(
                reply_runtime,
                query,
                busy_write_response::<ReloadNodeParametersResponse>(),
            );
        }
    }
}

fn spawn_reply<T>(reply_runtime: &Handle, query: Query, response: T)
where
    T: Message + Send + 'static,
    for<'a> <T as Message>::Codec: WireEncoder<Input<'a> = &'a T>,
{
    reply_runtime.spawn(async move {
        if let Err(err) = reply_async(query, response).await {
            tracing::warn!("[PARAM] Failed to send parameter reply: {err}");
        }
    });
}

async fn reply_async<T>(query: Query, response: T) -> std::result::Result<(), String>
where
    T: Message,
    for<'a> <T as Message>::Codec: WireEncoder<Input<'a> = &'a T>,
{
    let bytes = <<T as Message>::Codec as WireEncoder>::serialize(&response)
        .map_err(|error| format!("failed to serialize parameter reply: {error}"))?;
    let mut reply = query.reply(query.key_expr().clone(), bytes);
    if let Some(att_bytes) = query.attachment()
        && let Ok(att) = Attachment::try_from(att_bytes)
    {
        reply = reply.attachment(att);
    }
    reply
        .await
        .map_err(|err| format!("failed to send parameter reply: {err}"))
}

fn to_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use serde::{Deserialize, Serialize};

    use crate::{context::ContextBuilder, node::Node};

    use super::*;

    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    #[derive(Debug, Clone, Serialize, Deserialize, crate::Message)]
    #[message(name = "test_parameters::BusyCallbackParameters")]
    struct BusyCallbackParameters {
        enabled: bool,
    }

    #[test]
    fn busy_write_response_uses_stable_failure_payload() {
        let response = busy_write_response::<SetNodeParameterResponse>();
        assert!(!response.success);
        assert_eq!(response.message, PARAMETER_ACTOR_BUSY);
        assert_eq!(response.committed_revision, 0);
        assert!(response.changed_paths.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn callback_replies_busy_when_mailbox_is_full() -> TestResult {
        let (commands, _receiver) = mpsc::channel(1);
        let _permit = commands.clone().reserve_owned().await?;

        let response = call_set_service_with_commands("full", commands).await?;

        assert_busy_set_response(response);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn callback_replies_busy_when_mailbox_is_disconnected() -> TestResult {
        let (commands, receiver) = mpsc::channel(1);
        drop(receiver);

        let response = call_set_service_with_commands("disconnected", commands).await?;

        assert_busy_set_response(response);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn callback_replies_busy_for_atomic_when_mailbox_is_full() -> TestResult {
        let (commands, _receiver) = mpsc::channel(1);
        let _permit = commands.clone().reserve_owned().await?;

        let response = call_atomic_service_with_commands("atomic_full", commands).await?;

        assert_busy_atomic_response(response);
        Ok(())
    }

    async fn call_set_service_with_commands(
        suffix: &str,
        commands: mpsc::Sender<ParameterCommand<BusyCallbackParameters>>,
    ) -> TestResult<SetNodeParameterResponse> {
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .build()
            .await?;
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let server_name = format!("busy_parameter_server_{suffix}_{id}");
        let server_node = context.create_node(&server_name).build().await?;
        let service_name = format!("/{server_name}/parameter/set");
        let reply_runtime = Handle::current();
        let _server = register_server::<SetNodeParameterSrv>(&server_node, "~parameter/set", {
            let commands = commands.clone();
            let reply_runtime = reply_runtime.clone();
            move |query| {
                enqueue_or_reply_busy(
                    &commands,
                    &reply_runtime,
                    query,
                    |query| RemoteParameterCommand::Set { query },
                    busy_write_response::<SetNodeParameterResponse>,
                );
            }
        })
        .await?;

        let client_node = context
            .create_node(format!("busy_parameter_client_{suffix}_{id}"))
            .build()
            .await?;
        wait_for_service(&client_node, &service_name).await?;
        let client = client_node
            .create_service_client::<SetNodeParameterSrv>(&service_name)?
            .build()
            .await?;

        client
            .call_with_timeout_async(
                &SetNodeParameterRequest {
                    path: "enabled".to_string(),
                    value_json: "false".to_string(),
                    target_layer: "base".to_string(),
                    expected_revision: None,
                },
                Duration::from_secs(2),
            )
            .await
            .map_err(Into::into)
    }

    async fn call_atomic_service_with_commands(
        suffix: &str,
        commands: mpsc::Sender<ParameterCommand<BusyCallbackParameters>>,
    ) -> TestResult<SetNodeParametersAtomicallyResponse> {
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .build()
            .await?;
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let server_name = format!("busy_parameter_server_{suffix}_{id}");
        let server_node = context.create_node(&server_name).build().await?;
        let service_name = format!("/{server_name}/parameter/set_atomic");
        let reply_runtime = Handle::current();
        let _server = register_server::<SetNodeParametersAtomicallySrv>(
            &server_node,
            "~parameter/set_atomic",
            {
                let commands = commands.clone();
                let reply_runtime = reply_runtime.clone();
                move |query| {
                    enqueue_or_reply_busy(
                        &commands,
                        &reply_runtime,
                        query,
                        |query| RemoteParameterCommand::SetAtomic { query },
                        busy_write_response::<SetNodeParametersAtomicallyResponse>,
                    );
                }
            },
        )
        .await?;

        let client_node = context
            .create_node(format!("busy_parameter_client_{suffix}_{id}"))
            .build()
            .await?;
        wait_for_service(&client_node, &service_name).await?;
        let client = client_node
            .create_service_client::<SetNodeParametersAtomicallySrv>(&service_name)?
            .build()
            .await?;

        client
            .call_with_timeout_async(
                &SetNodeParametersAtomicallyRequest {
                    writes: vec![NodeParameterWriteJson {
                        path: "enabled".to_string(),
                        value_json: "false".to_string(),
                        target_layer: "base".to_string(),
                    }],
                    expected_revision: None,
                },
                Duration::from_secs(2),
            )
            .await
            .map_err(Into::into)
    }

    async fn wait_for_service(node: &Node, service: &str) -> TestResult {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        while start.elapsed() < timeout {
            if !node.graph().view().services_named(service).is_empty() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err(format!("timed out waiting for service {service}").into())
    }

    fn assert_busy_set_response(response: SetNodeParameterResponse) {
        assert!(!response.success);
        assert_eq!(response.message, PARAMETER_ACTOR_BUSY);
        assert_eq!(response.committed_revision, 0);
        assert!(response.changed_paths.is_empty());
    }

    fn assert_busy_atomic_response(response: SetNodeParametersAtomicallyResponse) {
        assert!(!response.success);
        assert_eq!(response.message, PARAMETER_ACTOR_BUSY);
        assert_eq!(response.committed_revision, 0);
        assert!(response.changed_paths.is_empty());
    }
}
