use std::{
    collections::{HashMap, hash_map::Entry},
    future::IntoFuture,
    sync::Arc,
    time::{Duration, Instant},
};

use booster::{RpcReqMsg, RpcRespMsg};
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{Result, WrapErr, eyre};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
enum RpcError {
    #[error("rpc response stream closed for {service_topic}")]
    ResponseStreamClosed { service_topic: &'static str },
    #[error(
        "rpc timed out for {service_topic} api {api_id} request {request_uuid} after {timeout:?}"
    )]
    Timeout {
        service_topic: &'static str,
        api_id: i32,
        request_uuid: String,
        timeout: Duration,
    },
    #[error(
        "rpc failed for {service_topic} api {api_id} request {request_uuid} with status {status}: {message}"
    )]
    Status {
        service_topic: &'static str,
        api_id: i32,
        request_uuid: String,
        status: i32,
        message: String,
    },
}

pub struct ZenohRpcClient {
    service_topic: &'static str,
    request_publisher: zenoh::pubsub::Publisher<'static>,
    pending_responses: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<RpcRespMsg>>>>,
    response_task: JoinHandle<()>,
    _session: zenoh::Session,
}

impl ZenohRpcClient {
    pub async fn new(session: &zenoh::Session, service_topic: &'static str) -> Result<Self> {
        let request_topic = format!("{service_topic}Req");
        let response_topic = format!("{service_topic}Resp");
        let request_publisher = session
            .declare_publisher(request_topic.clone())
            .await
            .map_err(|error| eyre!("failed to declare `{request_topic}` publisher: {error}"))?;
        let response_subscriber = session
            .declare_subscriber(response_topic.clone())
            .await
            .map_err(|error| eyre!("failed to declare `{response_topic}` subscriber: {error}"))?;
        let pending_responses = Arc::new(Mutex::new(HashMap::new()));
        let task_pending_responses = Arc::clone(&pending_responses);

        let response_task = tokio::spawn(async move {
            while let Ok(sample) = response_subscriber.recv_async().await {
                let payload = sample.payload().to_bytes();
                match decode_response(payload.as_ref()) {
                    Ok(response) => {
                        let response_uuid = response.uuid.clone();
                        let mut pending_responses = task_pending_responses.lock().await;
                        if !route_response_by_uuid(&mut pending_responses, response) {
                            debug!(
                                target: "booster_interface::rpc",
                                %response_topic,
                                response_uuid,
                                "ignore rpc response without pending waiter"
                            );
                        }
                    }
                    Err(error) => {
                        warn!(target: "booster_interface::rpc", %response_topic, error = %error, "failed to decode rpc response");
                    }
                }
            }
        });

        Ok(Self {
            service_topic,
            request_publisher,
            pending_responses,
            response_task,
            _session: session.clone(),
        })
    }

    pub async fn call(
        &self,
        api_id: i32,
        body: impl Into<String>,
        timeout: Duration,
    ) -> Result<()> {
        let deadline = Instant::now() + timeout;
        let request_uuid = Uuid::new_v4().to_string();
        let body = body.into();
        let payload = encode_request(&request_uuid, api_id, &body)?;
        let (response_sender, mut responses) = mpsc::unbounded_channel();

        self.pending_responses
            .lock()
            .await
            .insert(request_uuid.clone(), response_sender);

        debug!(
            target: "booster_interface::rpc",
            service_topic = self.service_topic,
            api_id,
            request_uuid,
            body,
            "send rpc request"
        );

        let result = async {
            wait_until_deadline(
                deadline,
                self.request_publisher.put(payload),
                RpcError::Timeout {
                    service_topic: self.service_topic,
                    api_id,
                    request_uuid: request_uuid.clone(),
                    timeout,
                },
            )
            .await?
            .map_err(|error| {
                eyre!(
                    "failed to publish `{}` rpc request: {error}",
                    self.service_topic
                )
            })?;

            loop {
                let response = wait_until_deadline(
                    deadline,
                    responses.recv(),
                    RpcError::Timeout {
                        service_topic: self.service_topic,
                        api_id,
                        request_uuid: request_uuid.clone(),
                        timeout,
                    },
                )
                .await?
                .ok_or(RpcError::ResponseStreamClosed {
                    service_topic: self.service_topic,
                })?;

                let status = parse_status_from_header(&response.header).unwrap_or(0);
                if status == -1 {
                    continue;
                }
                if status != 0 {
                    let message = if response.body.trim().is_empty() {
                        response.header
                    } else {
                        response.body
                    };
                    return Err(RpcError::Status {
                        service_topic: self.service_topic,
                        api_id,
                        request_uuid: request_uuid.clone(),
                        status,
                        message,
                    }
                    .into());
                }

                return Ok(());
            }
        }
        .await;

        self.pending_responses.lock().await.remove(&request_uuid);
        result
    }
}

async fn wait_until_deadline<T>(
    deadline: Instant,
    operation: impl IntoFuture<Output = T>,
    timeout_error: RpcError,
) -> Result<T> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    tokio::time::timeout(remaining, operation.into_future())
        .await
        .map_err(|_| timeout_error.into())
}

impl Drop for ZenohRpcClient {
    fn drop(&mut self) {
        self.response_task.abort();
    }
}

pub(crate) fn is_timeout_error(error: &color_eyre::Report) -> bool {
    matches!(
        error.downcast_ref::<RpcError>(),
        Some(RpcError::Timeout { .. })
    )
}

fn encode_request(uuid: &str, api_id: i32, body: &str) -> Result<Vec<u8>> {
    let request = RpcReqMsg {
        uuid: uuid.to_string(),
        header: serde_json::json!({ "api_id": api_id }).to_string(),
        body: body.to_string(),
    };
    cdr::serialize::<_, _, CdrLe>(&request, Infinite).wrap_err("failed to serialize rpc request")
}

fn decode_response(bytes: &[u8]) -> Result<RpcRespMsg> {
    cdr::deserialize(bytes).wrap_err("failed to deserialize rpc response")
}

fn route_response_by_uuid(
    pending: &mut HashMap<String, mpsc::UnboundedSender<RpcRespMsg>>,
    response: RpcRespMsg,
) -> bool {
    let response_uuid = response.uuid.clone();
    match pending.entry(response_uuid) {
        Entry::Occupied(entry) => {
            if entry.get().send(response).is_err() {
                entry.remove();
                return false;
            }
            true
        }
        Entry::Vacant(_) => false,
    }
}

fn parse_status_from_header(raw_json: &str) -> Option<i32> {
    let value: Value = serde_json::from_str(raw_json.trim()).ok()?;
    let object = value.as_object()?;
    object.get("status").and_then(parse_status_value)
}

fn parse_status_value(value: &Value) -> Option<i32> {
    match value {
        Value::Number(number) => number.as_i64().and_then(|value| i32::try_from(value).ok()),
        Value::String(string) => string.parse::<i32>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booster::RpcReqMsg;

    #[test]
    fn parse_status_reads_numeric_and_string_status_values() {
        assert_eq!(parse_status_from_header(r#"{"status":0}"#), Some(0));
        assert_eq!(parse_status_from_header(r#"{"status":"-1"}"#), Some(-1));
        assert_eq!(parse_status_from_header(r#"{"code":0}"#), None);
    }

    #[test]
    fn encode_request_round_trips_through_cdr() {
        let payload = encode_request("abc", 2001, r#"{"vx":0.1}"#).unwrap();
        let decoded: RpcReqMsg = cdr::deserialize(&payload).unwrap();

        assert_eq!(decoded.uuid, "abc");
        assert_eq!(decoded.header, r#"{"api_id":2001}"#);
        assert_eq!(decoded.body, r#"{"vx":0.1}"#);
    }

    #[test]
    fn route_response_by_uuid_only_delivers_to_matching_waiter() {
        let (first_sender, mut first_receiver) = mpsc::unbounded_channel();
        let (second_sender, mut second_receiver) = mpsc::unbounded_channel();
        let mut pending = std::collections::HashMap::from([
            ("first".to_string(), first_sender),
            ("second".to_string(), second_sender),
        ]);
        let response = RpcRespMsg {
            uuid: "second".to_string(),
            header: r#"{"status":0}"#.to_string(),
            body: "{}".to_string(),
        };

        route_response_by_uuid(&mut pending, response);

        assert!(first_receiver.try_recv().is_err());
        assert_eq!(second_receiver.try_recv().unwrap().uuid, "second");
    }

    #[tokio::test]
    async fn wait_until_deadline_times_out_pending_operation() {
        let timeout = Duration::from_millis(1);
        let result = wait_until_deadline(
            Instant::now() + timeout,
            std::future::pending::<()>(),
            RpcError::Timeout {
                service_topic: "test/topic",
                api_id: 42,
                request_uuid: "uuid".to_string(),
                timeout,
            },
        )
        .await;

        assert!(is_timeout_error(&result.unwrap_err()));
    }
}
