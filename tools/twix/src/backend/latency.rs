use std::time::{Duration, SystemTime};

use ros_z::time::Time;
use ros_z_debug::SampleRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SampleLatency {
    pub source_to_forward: Duration,
    pub transport_to_forward: Option<Duration>,
    pub receive_to_forward: Duration,
}

pub(crate) fn sample_latency<T>(record: &SampleRecord<T>, forward_time: Time) -> SampleLatency {
    SampleLatency {
        source_to_forward: record.source_latency_at(forward_time),
        transport_to_forward: record.transport_latency_at(forward_time),
        receive_to_forward: record.receive_latency_at(forward_time),
    }
}

pub(crate) fn trace_forward_latency<T>(kind: &'static str, record: &SampleRecord<T>) {
    if !tracing::enabled!(target: "twix::latency", tracing::Level::TRACE) {
        return;
    }

    let forward_time = Time::from_wallclock(SystemTime::now());
    let latency = sample_latency(record, forward_time);
    let transport_to_forward_ms = latency.transport_to_forward.map(duration_ms);

    tracing::trace!(
        target: "twix::latency",
        kind,
        topic = %record.metadata.resolved_topic,
        type_name = %record.metadata.type_info.name,
        sequence_number = record.publication_id.sequence_number(),
        source_to_forward_ms = duration_ms(latency.source_to_forward),
        ?transport_to_forward_ms,
        receive_to_forward_ms = duration_ms(latency.receive_to_forward),
        "forwarded subscription sample"
    );
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
