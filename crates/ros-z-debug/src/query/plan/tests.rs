use super::*;
use ros_z::Message;

struct Times(Vec<Time>);
impl TimeIndex for Times {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn time(&self, index: usize) -> Time {
        self.0[index]
    }
}
fn times(values: &[i64]) -> Times {
    Times(values.iter().copied().map(Time::from_nanos).collect())
}
fn source(name: &str) -> Source {
    TopicSource::new(name, TimeProjection::SourceTime).into()
}
fn exact_input(name: &str) -> InputSpec {
    InputSpec {
        source: source(name),
        selection: Selection::Exact,
    }
}
fn views(plan: &QueryPlan, data: &[(&str, &[i64])]) -> Vec<SourceState> {
    plan.sources()
        .unwrap()
        .iter()
        .map(|source| {
            if source.topic == "/pending" {
                return SourceState::Pending;
            }
            if source.topic == "/failed" {
                return SourceState::Failed("invalid projection".into());
            }
            let Some((_, records)) = data.iter().find(|(topic, _)| *topic == source.topic) else {
                return SourceState::Absent;
            };
            SourceState::Ready(SourceView {
                topic: TopicInfo {
                    name: source.topic.clone(),
                    type_info: u64::type_info(),
                    schema: None,
                    error: None,
                },
                index: Arc::new(times(records)),
                coverage: Coverage {
                    source: SourceCoverage::Complete,
                    truncated: false,
                },
                diagnostics: vec![],
            })
        })
        .collect()
}
fn evaluate(plan: &QueryPlan, data: &[(&str, &[i64])], cursor: i64) -> Evaluation {
    let views = views(plan, data);
    plan.evaluate(Time::from_nanos(cursor), &views, |source, slot| {
        let SourceState::Ready(view) = &views[source] else {
            panic!("unready load")
        };
        Ok(Arc::new(RawRecord {
            publication: None,
            id: slot as u64,
            time: view.index.time(slot),
            source_time: Time::from_nanos(1000),
            log_time: None,
            data: Arc::from([]),
        }))
    })
    .unwrap()
}

#[test]
fn selectors_share_boundary_duplicate_and_eligibility_rules() {
    let index = times(&[10, 10, 20, 30]);
    let at = Time::from_nanos(10);
    assert_eq!(
        select_indices(&index, at, &Selection::latest(), 10, |_| true).0,
        vec![1]
    );
    assert_eq!(
        select_indices(&index, at, &Selection::Exact, 10, |_| true).0,
        vec![1]
    );
    assert_eq!(
        select_indices(
            &index,
            at,
            &Selection::Window {
                before: Duration::ZERO,
                after: Duration::ZERO
            },
            10,
            |_| true
        )
        .0,
        vec![0, 1]
    );
    assert_eq!(
        select_indices(
            &index,
            at,
            &Selection::After {
                count: NonZeroUsize::MIN
            },
            10,
            |_| true
        )
        .0,
        vec![2]
    );
    assert_eq!(
        select_indices(
            &index,
            Time::from_nanos(20),
            &Selection::Bracket {
                max_gap: Duration::from_nanos(20)
            },
            10,
            |slot| slot != 2
        )
        .0,
        vec![1, 3]
    );
    assert!(
        select_indices(
            &index,
            Time::from_nanos(20),
            &Selection::Bracket {
                max_gap: Duration::from_nanos(19)
            },
            10,
            |slot| slot != 2
        )
        .0
        .is_empty()
    );
    assert_eq!(
        select_indices(&index, at, &Selection::Exact, 10, |slot| slot != 1).0,
        vec![0]
    );
}

#[test]
fn computed_timestamp_never_loads_payload_and_preserves_dependency_states() {
    let expression = available(
        [
            source("/objects"),
            source("/poses"),
            source("/pending"),
            source("/failed"),
        ]
        .into_iter()
        .map(|s| s.latest(cursor()).effective_time()),
    )
    .min();
    let plan = expression.plan();
    let views = views(&plan, &[("/objects", &[10, 20]), ("/poses", &[15])]);
    let result = plan
        .evaluate(Time::from_nanos(30), &views, |_, _| {
            panic!("timestamp query loaded payload")
        })
        .unwrap();
    assert!(matches!(result.state,State::Ready(Value::Time(Some(t))) if t==Time::from_nanos(15)));
    assert!(!result.settled());
    assert!(
        result
            .dependencies
            .iter()
            .any(|d| matches!(d.resolution, State::Failed(_)))
    );
    assert!(result.valid_for.contains(Time::from_nanos(35)));
}

#[test]
fn frame_plan_matches_available_data_and_fixed_plan_preserves_missing_detections() {
    let images = source("/images");
    let objects = source("/objects");
    let pose = source("/poses");
    let anchor = available([
        objects.latest(cursor()).effective_time(),
        pose.latest(cursor()).effective_time(),
    ])
    .min();
    let inputs = vec![
        exact_input("/objects"),
        InputSpec {
            source: source("/camera"),
            selection: Selection::Bracket {
                max_gap: Duration::from_nanos(10),
            },
        },
        exact_input("/failed"),
    ];
    let live = images
        .exact(anchor)
        .or_else(images.latest(cursor()))
        .attach(inputs.clone())
        .plan();
    let data: [(&str, &[i64]); 4] = [
        ("/images", &[10, 20, 30]),
        ("/objects", &[10, 20]),
        ("/poses", &[10]),
        ("/camera", &[5, 15, 25, 35]),
    ];
    let State::Ready(Value::Frames(frames)) = evaluate(&live, &data, 30).state else {
        panic!("frame")
    };
    assert_eq!(frames[0].primary.record.time, Time::from_nanos(10));
    assert!(
        matches!(&frames[0].inputs[0],State::Ready(records) if records.len()==1 && records[0].record.source_time>Time::from_nanos(30))
    );
    assert!(matches!(&frames[0].inputs[1],State::Ready(records) if records.len()==2));
    assert!(matches!(&frames[0].inputs[2], State::Failed(_)));
    let fixed = images.latest(cursor()).attach(inputs).plan();
    let State::Ready(Value::Frames(frames)) = evaluate(&fixed, &data, 30).state else {
        panic!("frame")
    };
    assert_eq!(frames[0].primary.record.time, Time::from_nanos(30));
    assert!(matches!(&frames[0].inputs[0],State::Ready(records) if records.is_empty()));
    let mut missing_image = data;
    missing_image[2] = ("/poses", &[15]);
    let State::Ready(Value::Frames(frames)) = evaluate(&live, &missing_image, 30).state else {
        panic!("frame")
    };
    assert_eq!(frames[0].primary.record.time, Time::from_nanos(30));
}

#[test]
fn source_presence_and_temporal_fallback_do_not_hide_pending_or_failed_sources() {
    let primary = TopicSource::new("/primary", TimeProjection::SourceTime);
    let fallback = TopicSource::new("/fallback", TimeProjection::SourceTime);
    let plan = Source::first_present([primary, fallback])
        .latest(cursor())
        .plan();
    assert!(
        matches!(evaluate(&plan,&[("/primary",&[]),("/fallback",&[10])],20).state,State::Ready(Value::Records(records)) if records.is_empty())
    );
    assert!(
        matches!(evaluate(&plan,&[("/fallback",&[10])],20).state,State::Ready(Value::Records(records)) if records.len()==1)
    );
    for (topic, pending) in [("/pending", true), ("/failed", false)] {
        let grouped = Source::first_present([
            TopicSource::new(topic, TimeProjection::SourceTime),
            TopicSource::new("/fallback", TimeProjection::SourceTime),
        ])
        .latest(cursor())
        .plan();
        let state = evaluate(&grouped, &[("/fallback", &[10])], 20).state;
        assert!(if pending {
            matches!(state, State::Pending)
        } else {
            matches!(state, State::Failed(_))
        });
        let plan = source(topic)
            .exact(cursor())
            .or_else(source("/fallback").latest(cursor()))
            .plan();
        let state = evaluate(&plan, &[("/fallback", &[10])], 20).state;
        assert!(if pending {
            matches!(state, State::Pending)
        } else {
            matches!(state, State::Failed(_))
        });
    }
}

#[test]
fn validity_ranges_preserve_real_plan_results_and_missing_input_diagnostics() {
    let max = i64::MAX;
    let primary = [0, 10, 10, 20, 30, max];
    let detections = [0, 10, 20, max];
    let camera = [0, 10, 30, max];
    let data: [(&str, &[i64]); 3] = [
        ("/primary", &primary),
        ("/detections", &detections),
        ("/camera", &camera),
    ];
    let inputs = vec![
        exact_input("/detections"),
        InputSpec {
            source: source("/camera"),
            selection: Selection::Bracket {
                max_gap: Duration::from_nanos(20),
            },
        },
    ];
    let mut selections = vec![
        Selection::AtOrBefore {
            count: NonZeroUsize::new(2).unwrap(),
        },
        Selection::After {
            count: NonZeroUsize::new(2).unwrap(),
        },
        Selection::Exact,
        Selection::Bracket {
            max_gap: Duration::from_nanos(20),
        },
    ];
    for before in [Duration::ZERO, Duration::from_nanos(3), Duration::MAX] {
        for after in [Duration::ZERO, Duration::from_nanos(7), Duration::MAX] {
            selections.push(Selection::Window { before, after });
        }
    }
    let cursors: Vec<_> = (0..42).chain([max - 1, max]).collect();
    for selection in selections {
        let query = source("/primary")
            .select(selection, cursor())
            .requiring(inputs.clone())
            .attach(inputs.clone())
            .plan();
        for &at in &cursors {
            let result = evaluate(&query, &data, at);
            assert!(result.valid_for.contains(Time::from_nanos(at)));
            for &other in &cursors {
                if result.valid_for.contains(Time::from_nanos(other)) {
                    let other = evaluate(&query, &data, other);
                    assert_eq!(
                        serde_json::to_value(&result.state).unwrap(),
                        serde_json::to_value(&other.state).unwrap()
                    );
                    assert_eq!(result.missing, other.missing);
                }
            }
        }
    }
}

#[test]
fn required_inputs_filter_before_counting_and_report_rejected_candidates() {
    let input = exact_input("/detections");
    let camera = exact_input("/camera");
    let query = source("/images")
        .select(
            Selection::AtOrBefore {
                count: NonZeroUsize::new(2).unwrap(),
            },
            cursor(),
        )
        .requiring([input.clone(), camera.clone()])
        .attach([input, camera])
        .plan();
    let result = evaluate(
        &query,
        &[
            ("/images", &[10, 20, 30]),
            ("/detections", &[10, 20, 20]),
            ("/camera", &[20]),
        ],
        30,
    );
    assert_eq!(
        result.missing,
        Some(MissingMatches {
            time: Time::from_nanos(30),
            inputs: vec![0, 1]
        })
    );
    let State::Ready(Value::Frames(frames)) = result.state else {
        panic!("frames")
    };
    // The later rejection at 10 must not replace the complete diagnostic at 30.
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].primary.record.time, Time::from_nanos(20));
    assert!(matches!(&frames[0].inputs[0],State::Ready(records) if records[0].record.id==2));
}
