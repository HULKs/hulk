use super::*;
use ros_z::{Message, dynamic::DynamicCdrCodec, message::WireEncoder};

fn payload<T: Message>(value: &T) -> DynamicPayload {
    let bytes = T::Codec::serialize(value).unwrap();
    DynamicCdrCodec::decode(&bytes, &Arc::new(T::schema())).unwrap()
}

#[test]
fn numeric_scalars_are_selected_without_json_or_string_coercion() {
    for (value, expected) in [
        (DynamicValue::Int8(-1), -1.0),
        (DynamicValue::Int16(-2), -2.0),
        (DynamicValue::Int32(-3), -3.0),
        (DynamicValue::Int64(-4), -4.0),
        (DynamicValue::Uint8(1), 1.0),
        (DynamicValue::Uint16(2), 2.0),
        (DynamicValue::Uint32(3), 3.0),
        (DynamicValue::Uint64(4), 4.0),
        (DynamicValue::Float32(0.5), 0.5),
        (DynamicValue::Float64(-0.75), -0.75),
        (
            DynamicValue::Optional(Some(Box::new(DynamicValue::Float64(2.0)))),
            2.0,
        ),
    ] {
        assert_eq!(
            numeric_value(SelectedValue::Value(&value)).unwrap(),
            expected
        );
    }
    assert_eq!(numeric_value(SelectedValue::Byte(255)).unwrap(), 255.0);
    for value in [
        DynamicValue::Bool(true),
        DynamicValue::String("12".into()),
        DynamicValue::Sequence(vec![]),
        DynamicValue::Optional(None),
        DynamicValue::Float64(f64::NAN),
        DynamicValue::Float64(f64::INFINITY),
        DynamicValue::Float64(f64::NEG_INFINITY),
    ] {
        assert!(numeric_value(SelectedValue::Value(&value)).is_err());
    }
}

#[test]
fn missing_optional_array_elements_and_nonfinite_values_break_lines() {
    let samples = [
        payload(&vec![Some(1.0_f64)]),
        payload(&vec![None::<f64>]),
        payload(&vec![Some(2.0_f64)]),
        payload(&Vec::<Option<f64>>::new()),
        payload(&vec![Some(3.0_f64)]),
        payload(&vec![Some(f64::NAN)]),
        payload(&vec![Some(4.0_f64)]),
        payload(&vec![Some(5.0_f64)]),
    ];
    let mut series = SeriesData::default();
    series.project_samples(
        &"[0]".parse().unwrap(),
        samples
            .iter()
            .enumerate()
            .map(|(index, payload)| (Time::from_nanos(index as i64), payload)),
    );
    assert_eq!(series.gaps, 3);
    assert!(series.issue.is_none());
    assert_eq!(
        series
            .segments
            .iter()
            .map(|segment| segment.iter().map(|(_, y)| *y).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0, 5.0]]
    );
}

#[test]
fn nested_detection_fields_use_the_same_paths_as_text() {
    use types::{
        object_detection::{Object, RobocupObjectLabel},
        time_wrapper::TimeWrapper,
    };
    let sample = payload(&TimeWrapper {
        time: Time::zero(),
        inner: vec![Object::<RobocupObjectLabel>::from([1.0, 2.0, 3.0, 4.0, 0.75, 0.0]); 3],
    });
    let mut series = SeriesData::default();
    series.project_samples(
        &"inner[2].bounding_box.confidence".parse().unwrap(),
        std::iter::once((Time::from_nanos(12), &sample)),
    );
    assert_eq!(series.segments, vec![vec![(Time::from_nanos(12), 0.75)]]);
    assert!(series.issue.is_none());
    let mut invalid = SeriesData::default();
    invalid.project_samples(
        &"inner[2].missing".parse().unwrap(),
        std::iter::once((Time::zero(), &sample)),
    );
    assert!(invalid.segments.is_empty());
    assert!(invalid.issue.unwrap().contains("missing"));
}

#[test]
fn source_time_offsets_preserve_precision() {
    let end = Time::from_nanos(1_700_000_000_000_000_000);
    assert_eq!(
        seconds_from(end.saturating_sub(Duration::from_micros(1)), end),
        -0.000001
    );
    assert_eq!(
        seconds_from(end.saturating_sub(Duration::from_secs(2)), end),
        -2.0
    );
    assert_eq!(seconds_from(end, end), 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_history_retains_interframe_samples_and_resume_reads_the_live_window() {
    use crate::{backend::RobotBackend, panel::PanelCreationContext};
    let backend = Arc::new(
        RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
            .await
            .unwrap(),
    );
    let publisher = backend
        .node()
        .publisher::<Vec<Option<f64>>>("/twix_plot_history_test")
        .build()
        .await
        .unwrap();
    let context = PanelCreationContext {
        backend: Arc::clone(&backend),
        value: None,
        egui_context: Default::default(),
    };
    let topic = "/twix_plot_history_test";
    let mut history = PlotHistory::default();
    history.reconcile(&context, [topic, topic].into_iter(), Duration::from_secs(5));
    assert_eq!(history.topics.len(), 1);

    // Wait for dynamic discovery before sending the measured sequence.
    tokio::time::timeout(Duration::from_secs(5), async {
        while history.latest(topic).is_none() {
            publisher
                .publish_with_source_time(&vec![Some(0.0), Some(0.0)], Time::zero())
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    for (seconds, value) in [
        (100, Some(1.0)),
        (101, None),
        (102, Some(2.0)),
        (103, Some(3.0)),
    ] {
        let time = Time::from_nanos(seconds * 1_000_000_000);
        publisher
            .publish_with_source_time(&vec![value, Some(7.0)], time)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while history.latest(topic).unwrap().source_time != time {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    history.refresh();
    let mut series = SeriesData::default();
    history.project(topic, "[0]", &mut series);
    assert_eq!(series.gaps, 1);
    assert_eq!(
        series.segments.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![1, 2]
    );
    let frozen = series.records.clone().unwrap();
    assert_eq!(frozen.len(), 4);

    history.reconcile(&context, [topic, topic].into_iter(), Duration::from_secs(5));
    history.project(topic, "[1]", &mut series);
    assert!(Arc::ptr_eq(series.records.as_ref().unwrap(), &frozen));
    assert_eq!(
        series.segments[0]
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<_>>(),
        vec![7.0; 4]
    );

    // Paused views keep their snapshot even after samples expire from the live buffer.
    let next = Time::from_nanos(110_000_000_000);
    publisher
        .publish_with_source_time(&vec![Some(9.0), Some(8.0)], next)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while history.latest(topic).unwrap().source_time != next {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    history.project(topic, "[0]", &mut series);
    assert!(Arc::ptr_eq(series.records.as_ref().unwrap(), &frozen));
    assert_eq!(history.end_time(), Some(Time::from_nanos(103_000_000_000)));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            history.refresh();
            if history.end_time() == Some(next) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    history.project(topic, "[0]", &mut series);
    assert_eq!(series.segments, vec![vec![(next, 9.0)]]);
    assert_eq!(history.end_time(), Some(next));

    backend.set_namespace("/another_robot".into()).unwrap();
    assert!(history.reconcile(&context, [topic].into_iter(), Duration::from_secs(5)));
    history.project(topic, "[0]", &mut series);
    assert!(series.segments.is_empty());
    history.reconcile(&context, std::iter::empty(), Duration::from_secs(5));
    assert!(history.topics.is_empty());
}
