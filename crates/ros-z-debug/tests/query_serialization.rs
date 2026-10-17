use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    pubsub::QueueOverflowReporting,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
};
use ros_z_debug::{
    ObservationPolicy, RetentionPolicy,
    query::{
        TimeProjection,
        plan::{self, QueryPlan, Source, TopicSource},
    },
};

#[test]
fn query_plan_round_trip_preserves_live_policy() {
    let policy = ObservationPolicy::time_window(Duration::from_secs(5))
        .unwrap()
        .with_subscriber_qos(QosProfile {
            reliability: QosReliability::BestEffort,
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::new(7).unwrap()),
            ..Default::default()
        })
        .with_subscriber_queue_capacity(NonZeroUsize::new(13).unwrap())
        .with_update_buffer_capacity(NonZeroUsize::new(17).unwrap())
        .with_queue_overflow_reporting(QueueOverflowReporting::Silent);
    let source = TopicSource {
        topic: "/test".into(),
        projection: TimeProjection::SourceTime,
        live_policy: policy,
    };
    let plan = Source::from(source.clone()).latest(plan::cursor()).plan();
    let json = serde_json::to_string(&plan).unwrap();
    let decoded: QueryPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.sources().unwrap(), vec![source]);
}

#[test]
fn missing_live_policy_uses_default() {
    let source = TopicSource::new("/test", TimeProjection::SourceTime);
    let mut json = serde_json::to_value(&source).unwrap();
    json.as_object_mut().unwrap().remove("live_policy");
    assert_eq!(serde_json::from_value::<TopicSource>(json).unwrap(), source);
}

#[test]
fn retention_deserialization_rejects_zero_duration() {
    let policy = RetentionPolicy::time_window(Duration::from_secs(1)).unwrap();
    let mut json = serde_json::to_value(policy).unwrap();
    json["TimeWindow"]["duration"] = serde_json::to_value(Duration::ZERO).unwrap();
    assert!(serde_json::from_value::<RetentionPolicy>(json).is_err());
}
