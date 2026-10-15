use std::{borrow::Cow, collections::BTreeMap, sync::Arc, time::Duration};

use super::recording::{QueryRecording, Recording};
use ros_z::time::Time;
use ros_z::{Message, message::WireEncoder};
use ros_z_debug::query::plan::{
    self, Evaluation, Frame, InputSpec, QueryPlan, Selection, Source, State, TopicSource, Value,
};
use ros_z_debug::query::{TimeEncoding, TimeProjection};
use std::sync::atomic::AtomicBool;

fn channels<T: Message>(topics: &[&str]) -> Vec<Arc<mcap::Channel<'static>>> {
    let bundle = T::schema();
    let hash = ros_z_schema::compute_hash(&bundle).unwrap();
    let schema = Arc::new(mcap::Schema {
        id: 1,
        name: T::type_name(),
        encoding: "ros-z-schema-json".into(),
        data: Cow::Owned(serde_json::to_vec(&bundle).unwrap()),
    });
    topics
        .iter()
        .enumerate()
        .map(|(id, topic)| {
            Arc::new(mcap::Channel {
                id: id as u16 + 1,
                topic: (*topic).into(),
                schema: Some(schema.clone()),
                message_encoding: "ros-z-cdr".into(),
                metadata: BTreeMap::from([
                    ("ros_z.type_name".into(), T::type_name()),
                    ("ros_z.schema_hash".into(), hash.to_hash_string()),
                ]),
            })
        })
        .collect()
}

pub(crate) fn fixture_records(
    message_indexes: bool,
    flush_each: bool,
    records: &[(usize, u64, u64, u64)],
) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let channels = channels::<Time>(&["images", "detections"]);
    let mut writer = mcap::WriteOptions::new()
        .chunk_size(None)
        .emit_message_indexes(message_indexes)
        .create(file.reopen().unwrap())
        .unwrap();
    for (sequence, &(channel, stamp, source, log)) in records.iter().enumerate() {
        writer
            .write(&mcap::Message {
                channel: channels[channel].clone(),
                sequence: sequence as u32,
                log_time: log,
                publish_time: source,
                data: Cow::Owned(
                    <Time as Message>::Codec::serialize(
                        &Time::zero().saturating_add(Duration::from_nanos(stamp)),
                    )
                    .unwrap(),
                ),
            })
            .unwrap();
        if flush_each {
            writer.flush().unwrap();
        }
    }
    writer.finish().unwrap();
    file
}

fn open(file: &std::path::Path) -> QueryRecording {
    QueryRecording::new(
        Recording::open(file, "/replay").unwrap(),
        "fixture".into(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}

fn projected(topic: &str) -> TopicSource {
    TopicSource::new(
        format!("/replay/{topic}"),
        TimeProjection::field("", TimeEncoding::RosZTime).unwrap(),
    )
}

fn finish(recording: &mut QueryRecording, query: &QueryPlan, time: Time) -> Evaluation {
    let mut result = None;
    recording
        .query_plan(query, time, |evaluation| {
            result = Some(evaluation);
            true
        })
        .unwrap();
    let result = result.unwrap();
    assert!(result.settled());
    result
}

fn frames(result: &Evaluation) -> &[Frame] {
    let State::Ready(Value::Frames(frames)) = &result.state else {
        panic!("expected frames: {result:?}")
    };
    frames
}

fn image_plan(selection: Selection, matching: Selection) -> QueryPlan {
    let input = InputSpec {
        source: projected("detections").into(),
        selection: matching,
    };
    Source::from(projected("images"))
        .select(selection, plan::cursor())
        .requiring([input.clone()])
        .attach([input])
        .plan()
}

#[test]
fn progressive_indexes_preserve_file_order_ties_and_reuse_loaded_chunks() {
    for message_indexes in [false, true] {
        let file = fixture_records(
            message_indexes,
            false,
            &[
                (0, 20, 220, 400),
                (0, 10, 110, 300),
                (1, 10, 800, 100),
                (1, 20, 900, 200),
                (0, 20, 225, 50),
                (1, 20, 905, 25),
            ],
        );
        let mut recording = open(file.path());
        let plan = image_plan(Selection::latest(), Selection::Exact);
        let result = finish(&mut recording, &plan, Time::from_nanos(20));
        let frame = &frames(&result)[0];
        assert_eq!(frame.primary.record.source_time, Time::from_nanos(225));
        let State::Ready(inputs) = &frame.inputs[0] else {
            panic!("matched inputs")
        };
        assert_eq!(inputs[0].record.source_time, Time::from_nanos(905));
        assert_eq!(
            result.bounds,
            Some((Time::from_nanos(10), Time::from_nanos(20)))
        );
        assert_eq!(recording.decoded_chunks(), 3);
        assert_eq!(recording.index_stats(), (2, 6 * 16));
        finish(&mut recording, &plan, Time::from_nanos(10));
        assert_eq!(recording.decoded_chunks(), 3);
        assert_eq!(recording.projected_messages, 6);
    }
}

#[test]
fn index_budget_evicts_lru_and_pins_active_plan_sources() {
    let file = fixture_records(
        true,
        true,
        &[
            (0, 10, 110, 210),
            (0, 20, 120, 220),
            (0, 30, 130, 230),
            (1, 20, 520, 920),
            (1, 10, 510, 930),
        ],
    );
    let raw = TopicSource::new("/replay/images", TimeProjection::SourceTime);
    let images = projected("images");
    let detections = projected("detections");
    let query = |source: TopicSource| Source::from(source).latest(plan::cursor()).plan();
    let time = Time::from_nanos(1000);
    let mut recording = open(file.path()).with_index_budget(96).unwrap();
    for source in [
        images.clone(),
        raw.clone(),
        images.clone(),
        detections.clone(),
    ] {
        finish(&mut recording, &query(source), time);
    }
    assert!(recording.has_index(&images));
    assert!(!recording.has_index(&raw));
    assert!(recording.has_index(&detections));
    assert!(recording.index_stats().1 <= 96);
    let mut recording = open(file.path()).with_index_budget(64).unwrap();
    finish(&mut recording, &query(images.clone()), time);
    let result = finish(
        &mut recording,
        &image_plan(Selection::latest(), Selection::Exact),
        time,
    );
    assert!(matches!(result.state, State::Failed(error) if error.contains("--index-cache-mib")));
    assert!(recording.has_index(&images));
    assert!(!recording.has_index(&detections));
    assert_eq!(recording.index_stats(), (1, 48));
}
