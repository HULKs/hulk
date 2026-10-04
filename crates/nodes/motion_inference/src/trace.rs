use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use ros_z::{
    Message,
    prelude::Publisher,
    pubsub::Received,
    service::RequestId,
    time::{Clock, Time},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Receiver, Sender, error::TrySendError};

use crate::{config::Policy, inference::InferenceCommand, node::InferenceError};

pub const TRACE_TOPIC: &str = "motion_inference/trace";

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct PublicationOrigin {
    pub topic: String,
    pub publication_id: String,
    pub source_time: Time,
    pub received_at: Time,
}

impl PublicationOrigin {
    pub(crate) fn from_received<T>(topic: &str, value: &Received<T>, received_at: Time) -> Self {
        Self {
            topic: topic.into(),
            publication_id: value.publication_id().to_string(),
            source_time: value.source_time,
            received_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct InputSources {
    pub sensor: PublicationOrigin,
    pub previous_velocity_sensor: Option<String>,
    pub joint_limits: PublicationOrigin,
    pub last_commanded_request: Option<RequestId>,
    pub configuration: String,
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct Record {
    // Event time; the MCAP source timestamp reflects later telemetry publication.
    pub time: Time,
    // Undecodable requests have no recoverable service ID at this layer.
    pub request_id: Option<RequestId>,
    // Local queue/publication failures as of publication, not transport/recorder losses.
    pub dropped_records: u64,
    pub event: Event,
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub enum Event {
    Request(InferenceCommand),
    DecodeError {
        service: String,
        error: String,
    },
    Sensor {
        source: PublicationOrigin,
        velocity_updated: bool,
    },
    Input {
        policy: Policy,
        shape: Vec<i64>,
        values: Vec<f32>,
        sources: Option<Arc<InputSources>>,
        inference_time: Time,
        // None marks initialization; otherwise links retained state to the preceding inference.
        previous_request: Option<RequestId>,
    },
    Output {
        policy: Policy,
        shape: Vec<i64>,
        values: Vec<f32>,
    },
    // The service result, not confirmation that the caller received it.
    Outcome {
        error: Option<InferenceError>,
    },
}

#[derive(Clone)]
pub(crate) struct Trace {
    request_id: Option<RequestId>,
    inputs: Option<Arc<InputSources>>,
    clock: Clock,
    publisher: Arc<Publisher<Record>>,
    sender: Sender<Record>,
    dropped: Arc<AtomicU64>,
}

impl Trace {
    pub fn channel(clock: Clock, publisher: Publisher<Record>) -> (Self, Receiver<Record>) {
        // Never backpressure service callbacks or the inference worker on telemetry.
        let (sender, receiver) = mpsc::channel(256);
        (
            Self {
                request_id: None,
                inputs: None,
                clock,
                publisher: Arc::new(publisher),
                sender,
                dropped: Arc::new(AtomicU64::new(0)),
            },
            receiver,
        )
    }

    pub fn for_request(&self, request_id: RequestId) -> Self {
        Self {
            request_id: Some(request_id),
            ..self.clone()
        }
    }

    pub fn with_inputs(&self, sources: InputSources) -> Self {
        Self {
            inputs: Some(Arc::new(sources)),
            ..self.clone()
        }
    }

    pub fn request_id(&self) -> Option<&RequestId> {
        self.request_id.as_ref()
    }

    pub fn inputs(&self) -> Option<&Arc<InputSources>> {
        self.inputs.as_ref()
    }

    pub fn record(&self, event: Event) {
        self.record_at(self.clock.now(), event);
    }

    pub fn record_at(&self, time: Time, event: Event) {
        let record = Record {
            time,
            request_id: self.request_id.clone(),
            dropped_records: 0,
            event,
        };
        if self.sender.try_send(record).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_lazy(&self, event: impl FnOnce() -> Event) {
        if try_record_lazy(&self.sender, self.publisher.has_subscribers(), || Record {
            time: self.clock.now(),
            request_id: self.request_id.clone(),
            dropped_records: 0,
            event: event(),
        })
        .is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn publish(&self, mut records: Receiver<Record>) {
        while let Some(mut record) = records.recv().await {
            if self
                .publisher
                .publish_if_subscribed(|| async {
                    record.dropped_records = self.dropped.load(Ordering::Relaxed);
                    record
                })
                .await
                .is_err()
            {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

fn try_record_lazy(
    sender: &Sender<Record>,
    subscribed: bool,
    build: impl FnOnce() -> Record,
) -> Result<(), TrySendError<()>> {
    if subscribed {
        sender.try_reserve()?.send(build());
    }
    Ok(())
}
