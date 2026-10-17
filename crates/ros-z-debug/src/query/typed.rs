use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};

use ros_z::{
    Message, SchemaHash, TypeInfo,
    dynamic::{DynamicCdrCodec, DynamicPayload},
    message::WireDecoder,
    pubsub::Received,
    time::Time,
};
use tokio_util::sync::CancellationToken;

use super::{
    Backend, CursorHandle, MAX_ROWS, PlanObservation, RawRecord, TimeProjection, TopicInfo,
    plan::{self, Sample, Selection, Source, State, TopicSource, Value},
    query_error,
};
use crate::{
    CachedSubscription, CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot,
    ObservationPolicy, Result, RetentionPolicy, SampleMetadata, SampleRecord, TargetIdentity,
    TopicReference, cache::CachedSubscriptionState,
};

type Decoded<T> = std::result::Result<Arc<SampleRecord<T>>, String>;
type RecordKey = (u64, Option<([u8; 16], i64)>, Time);

fn record_key(sample: &Sample) -> RecordKey {
    (
        sample.record.id,
        sample.record.publication,
        sample.record.time,
    )
}

// Visit every row before collecting errors, preserving good entries even when
// an earlier row fails. Both successes and failures live only as long as rows do.
fn decode_samples<T>(
    decoded: &mut HashMap<RecordKey, Decoded<T>>,
    samples: &[Sample],
    mut decode: impl FnMut(&Sample) -> Decoded<T>,
) -> std::result::Result<Vec<Arc<SampleRecord<T>>>, String> {
    let mut next = HashMap::with_capacity(samples.len());
    for sample in samples {
        let key = record_key(sample);
        next.entry(key)
            .or_insert_with(|| decoded.remove(&key).unwrap_or_else(|| decode(sample)));
    }
    *decoded = next;
    samples
        .iter()
        .map(|sample| decoded[&record_key(sample)].clone())
        .collect()
}

#[derive(Clone)]
pub struct Connection {
    pub backend: Arc<dyn Backend>,
    pub cursor: CursorHandle,
}

pub fn decode_record<T: Message>(
    record: &RawRecord,
    topic: &TopicInfo,
) -> Result<Arc<SampleRecord<T>>> {
    let value = decode_value::<T>(record, topic)?;
    Ok(wrap_record(
        record,
        topic,
        TopicReference::new(&topic.name)?,
        value,
    ))
}

fn decode_value<T: Message>(record: &RawRecord, topic: &TopicInfo) -> Result<T> {
    if topic.type_info != T::type_info() {
        return Err(query_error(format!(
            "{}: recorded schema does not match {}",
            topic.name,
            T::type_name()
        )));
    }
    T::Codec::deserialize(&record.data).map_err(query_error)
}

fn wrap_record<T>(
    record: &RawRecord,
    topic: &TopicInfo,
    topic_reference: TopicReference,
    value: T,
) -> Arc<SampleRecord<T>> {
    let (publisher, sequence) = record.publication.unwrap_or(([0; 16], record.id as i64));
    Arc::new(SampleRecord {
        value,
        source_time: record.source_time,
        effective_time: record.time,
        transport_time: record.log_time,
        publication_id: Received {
            message: (),
            transport_time: record.log_time,
            source_time: record.source_time,
            sequence_number: sequence,
            source_global_id: publisher.into(),
        }
        .publication_id(),
        metadata: Arc::new(SampleMetadata {
            topic_reference,
            resolved_topic: topic.name.clone(),
            type_info: topic.type_info.clone(),
        }),
    })
}

pub(crate) fn typed_cache<T: Message>(
    connection: &Connection,
    topic: TopicReference,
    identity: &TargetIdentity,
    policy: ObservationPolicy,
    projection: TimeProjection,
) -> Result<CachedSubscription<T>> {
    cache(
        connection,
        topic,
        identity,
        policy,
        projection,
        decode_value::<T>,
    )
}

pub(crate) fn dynamic_cache(
    connection: &Connection,
    topic: TopicReference,
    identity: &TargetIdentity,
    policy: ObservationPolicy,
    projection: TimeProjection,
) -> Result<CachedSubscription<DynamicPayload>> {
    cache(
        connection,
        topic,
        identity,
        policy,
        projection,
        |record, topic| {
            let schema = topic
                .schema
                .as_ref()
                .ok_or_else(|| query_error("topic schema unavailable"))?;
            DynamicCdrCodec::decode(&record.data, schema).map_err(query_error)
        },
    )
}

fn cache<T: Send + Sync + 'static>(
    connection: &Connection,
    topic: TopicReference,
    identity: &TargetIdentity,
    policy: ObservationPolicy,
    projection: TimeProjection,
    decode: impl Fn(&RawRecord, &TopicInfo) -> Result<T> + Send + 'static,
) -> Result<CachedSubscription<T>> {
    let resolved_topic = topic.resolve(identity)?;
    let selection = match policy.retention() {
        RetentionPolicy::LatestOnly => Selection::latest(),
        RetentionPolicy::TimeWindow(window) => Selection::Window {
            before: window.duration(),
            after: Duration::ZERO,
        },
    };
    let source = Source::from(TopicSource {
        topic: resolved_topic.clone(),
        expected_type: None,
        live_policy: policy,
        projection,
    });
    let mut plan = source.select(selection, plan::cursor()).plan();
    if let RetentionPolicy::TimeWindow(window) = policy.retention()
        && let Some(limit) = window.max_samples()
        && limit.get() < MAX_ROWS
    {
        plan = plan.with_max_rows(limit);
    }
    let raw = connection
        .backend
        .observe(connection.cursor.clone(), plan)?;
    let state = Arc::new(CachedSubscriptionState::new(
        CachedSubscriptionStatusSnapshot::with_metadata(
            CachedSubscriptionStatus::WaitingForFirstSample,
            resolved_topic,
            TypeInfo::new("pending", SchemaHash::zero()),
        ),
        policy.retention(),
        policy.update_buffer_capacity(),
    ));
    state.attach_query(raw.clone());
    let cursor = connection.cursor.clone();
    state.spawn_receive(move |state, cancellation| {
        receive_cache(raw, cursor, topic, decode, state, cancellation)
    });
    Ok(state.handle())
}

async fn receive_cache<T: Send + Sync + 'static>(
    raw: PlanObservation,
    cursor: CursorHandle,
    topic: TopicReference,
    decode: impl Fn(&RawRecord, &TopicInfo) -> Result<T>,
    state: Weak<CachedSubscriptionState<T>>,
    cancellation: CancellationToken,
) {
    let mut updates = raw.subscribe_updates();
    let mut positions = cursor.subscribe();
    let mut decoded = HashMap::new();
    let mut decoded_type = None;
    let mut applied: Option<(Vec<RecordKey>, u64)> = None;
    loop {
        positions.borrow_and_update();
        updates.borrow_and_update();
        let snapshot = raw.snapshot();
        if let Some(state) = state.upgrade() {
            if !raw.is_current(&snapshot)
                || matches!(snapshot.evaluation.state, State::Pending)
                || !snapshot.evaluation.settled()
            {
                // Keep the last resolved answer until its replacement is definitive.
            } else if let State::Failed(error) = &snapshot.evaluation.state {
                applied = None;
                state.install_query_result(Err(error.clone()), snapshot.revision);
            } else if let State::Ready(Value::Records(samples)) = &snapshot.evaluation.state {
                let sample_type = samples
                    .first()
                    .map(|sample| (&sample.topic.type_info, sample.topic.schema.is_some()));
                if sample_type.is_some()
                    && decoded_type.as_ref().map(|(info, schema)| (info, *schema)) != sample_type
                {
                    decoded.clear();
                    decoded_type = sample_type.map(|(info, schema)| (info.clone(), schema));
                    applied = None;
                }
                let unchanged = applied.as_ref().is_some_and(|(keys, revision)| {
                    *revision == snapshot.revision
                        && keys.iter().copied().eq(samples.iter().map(record_key))
                });
                if !unchanged {
                    let records = decode_samples(&mut decoded, samples, |sample| {
                        decode(&sample.record, &sample.topic)
                            .map(|value| {
                                wrap_record(&sample.record, &sample.topic, topic.clone(), value)
                            })
                            .map_err(|error| error.to_string())
                    });
                    state.install_query_result(records, snapshot.revision);
                    applied = Some((samples.iter().map(record_key).collect(), snapshot.revision));
                }
            }
        } else {
            break;
        }
        tokio::select! {
            _ = cancellation.cancelled() => break,
            changed = updates.changed() => if changed.is_err() { break; },
            changed = positions.changed() => if changed.is_err() { break; },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use parking_lot::Mutex;
    use tokio::sync::{oneshot, watch};

    use super::*;
    use crate::query::{
        Cursor, CursorRange, PlanObservation, PlanSnapshot, TopicList, TopicSnapshot,
        plan::{Dependency, Evaluation},
    };

    fn ready_result() -> Arc<Evaluation> {
        Arc::new(Evaluation {
            state: State::Ready(Value::Records(vec![Sample {
                record: Arc::new(RawRecord {
                    id: 7,
                    publication: None,
                    time: Time::from_nanos(10),
                    source_time: Time::from_nanos(10),
                    log_time: None,
                    data: Arc::from([]),
                }),
                topic: TopicInfo {
                    name: "/test".into(),
                    type_info: u64::type_info(),
                    schema: None,
                    error: None,
                },
            }])),
            cursor: Time::from_nanos(100),
            valid_for: CursorRange {
                start: Time::zero(),
                end: None,
            },
            bounds: None,
            missing: None,
            dependencies: Vec::new(),
        })
    }

    struct TestBackend(PlanObservation);

    fn channel(cursor: CursorHandle) -> (PlanObservation, impl Fn(Arc<PlanSnapshot>)) {
        let (sender, receiver) = watch::channel(Arc::new(PlanSnapshot {
            revision: cursor.position().revision,
            evaluation: Arc::new(Evaluation::unresolved(Time::zero(), State::Pending)),
        }));
        let (notify, updates) = watch::channel(0);
        (
            PlanObservation::new(cursor, move || receiver.borrow().clone(), updates),
            move |snapshot| {
                sender.send_replace(snapshot);
                notify.send_modify(|revision| *revision += 1);
            },
        )
    }

    impl Backend for TestBackend {
        fn topics(&self) -> TopicList {
            TopicList(watch::channel(Arc::new(TopicSnapshot::default())).1)
        }

        fn observe(&self, _: CursorHandle, plan: plan::QueryPlan) -> Result<PlanObservation> {
            plan.validate()?;
            Ok(self.0.clone())
        }
    }

    async fn settle() {
        // Let the worker consume watch notifications before checking for silence.
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn retention_caps_above_the_response_limit_still_start() {
        for limit in [2, MAX_ROWS, 5_000] {
            let cursor = CursorHandle::new(Cursor::Live);
            let (raw, _) = channel(cursor.clone());
            let connection = Connection {
                backend: Arc::new(TestBackend(raw)),
                cursor,
            };
            let retention = RetentionPolicy::time_window_with_max_samples(
                Duration::from_secs(1),
                NonZeroUsize::new(limit).unwrap(),
            )
            .unwrap();
            typed_cache::<u64>(
                &connection,
                TopicReference::new("/test").unwrap(),
                &TargetIdentity::new("/").unwrap(),
                ObservationPolicy::default().with_retention(retention),
                TimeProjection::default(),
            )
            .unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seek_during_first_decode_rejects_the_obsolete_installation() {
        let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(100)));
        for fail in [false, true] {
            let (raw, send) = channel(cursor.clone());
            let result = ready_result();
            let revision = cursor.position().revision;
            send(Arc::new(PlanSnapshot {
                revision,
                evaluation: result.clone(),
            }));
            let (started, waiting) = oneshot::channel();
            let started = Mutex::new(Some(started));
            let (release, gate) = std::sync::mpsc::channel();
            let gate = Mutex::new(gate);
            let cache = cache(
                &Connection {
                    backend: Arc::new(TestBackend(raw)),
                    cursor: cursor.clone(),
                },
                TopicReference::new("/test").unwrap(),
                &TargetIdentity::new("/").unwrap(),
                ObservationPolicy::default(),
                TimeProjection::SourceTime,
                move |record, _| {
                    if let Some(started) = started.lock().take() {
                        started.send(()).unwrap();
                        gate.lock().recv_timeout(Duration::from_secs(5)).unwrap();
                    }
                    if fail {
                        Err(query_error("obsolete decode error"))
                    } else {
                        Ok(record.id)
                    }
                },
            )
            .unwrap();
            assert!(
                cache.query_status().is_some(),
                "query guard is installed before the worker starts"
            );
            let mut updates = cache.subscribe_updates().unwrap();
            tokio::time::timeout(Duration::from_secs(3), waiting)
                .await
                .unwrap()
                .unwrap();
            cursor.set(Cursor::At(Time::from_nanos(50 + i64::from(fail))));
            send(Arc::new(PlanSnapshot {
                revision: cursor.position().revision,
                evaluation: Arc::new(Evaluation {
                    state: State::Pending,
                    ..(*result).clone()
                }),
            }));
            release.send(()).unwrap();
            settle().await;
            assert!(cache.latest().is_none());
            assert!(matches!(updates.try_recv(), Ok(None)));
            assert_eq!(
                cache.status().status(),
                &CachedSubscriptionStatus::WaitingForFirstSample
            );
        }
    }

    #[tokio::test]
    async fn pending_queries_retain_results_until_a_definitive_replacement() {
        let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(100)));
        let (raw, send) = channel(cursor.clone());
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let cache = cache(
            &Connection {
                backend: Arc::new(TestBackend(raw)),
                cursor: cursor.clone(),
            },
            TopicReference::new("/test").unwrap(),
            &TargetIdentity::new("/").unwrap(),
            ObservationPolicy::time_window(Duration::from_secs(1)).unwrap(),
            TimeProjection::default(),
            move |record, _| {
                counter.fetch_add(1, Ordering::Relaxed);
                if record.id == 8 {
                    return Err(query_error("bad row"));
                }
                Ok(record.id)
            },
        )
        .unwrap();
        let result = ready_result();
        let emit = |revision, value: Option<Arc<Evaluation>>| {
            send(Arc::new(PlanSnapshot {
                revision,
                evaluation: value.unwrap_or_else(|| {
                    Arc::new(Evaluation {
                        state: State::Pending,
                        ..(*result).clone()
                    })
                }),
            }));
        };
        emit(0, Some(Arc::clone(&result)));
        settle().await;
        let first = cache.latest().unwrap();
        let mut updates = cache.subscribe_updates().unwrap();
        cursor.advance(Time::from_nanos(101));
        settle().await;
        assert!(matches!(updates.try_recv(), Ok(None)));
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        let mut failing = (*result).clone();
        let State::Ready(Value::Records(samples)) = &mut failing.state else {
            unreachable!()
        };
        for id in [8, 9] {
            samples.push(Sample {
                record: Arc::new(RawRecord {
                    id,
                    ..(*samples[0].record).clone()
                }),
                topic: samples[0].topic.clone(),
            });
        }
        emit(0, Some(Arc::new(failing.clone())));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(cache.status().message(), Some("query: bad row"));
        let mut errors = cache.subscribe_updates().unwrap();
        let State::Ready(Value::Records(samples)) = &mut failing.state else {
            unreachable!()
        };
        samples.remove(0);
        // Changing the selected set must reuse both failed and successful decodes.
        emit(0, Some(Arc::new(failing.clone())));
        settle().await;
        assert!(matches!(errors.try_recv(), Ok(None)));
        let State::Ready(Value::Records(samples)) = &mut failing.state else {
            unreachable!()
        };
        samples.remove(0);
        emit(0, Some(Arc::new(failing)));
        settle().await;
        assert_eq!(
            cache.latest().unwrap().value,
            9,
            "good rows after errors are decoded and cached"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        emit(0, Some(Arc::clone(&result)));
        settle().await;
        assert_eq!(cache.status().status(), &CachedSubscriptionStatus::Ready);
        let first = cache.latest().unwrap();
        assert_eq!(
            calls.load(Ordering::Relaxed),
            4,
            "removed rows leave the decode cache"
        );

        // Pending responses preserve both data and the installation guard.
        let mut updates = cache.subscribe_updates().unwrap();
        emit(0, None);
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert!(matches!(updates.try_recv(), Ok(None)));
        emit(0, Some(Arc::clone(&result)));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert!(matches!(updates.try_recv(), Ok(None)));

        let empty = Evaluation {
            state: State::Ready(Value::Records(Vec::new())),
            ..(*result).clone()
        };
        let mut provisional = empty.clone();
        provisional.dependencies.push(Dependency {
            source: TopicSource {
                topic: "/test".into(),
                expected_type: None,
                live_policy: ObservationPolicy::default(),
                projection: TimeProjection::default(),
            },
            resolution: State::Pending,
            coverage: None,
            diagnostics: Vec::new(),
        });
        emit(0, Some(Arc::new(provisional)));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert!(matches!(updates.try_recv(), Ok(None)));

        emit(0, Some(Arc::new(empty)));
        settle().await;
        assert!(cache.latest().is_none());
        emit(0, Some(Arc::clone(&result)));
        settle().await;
        assert_eq!(cache.latest().unwrap().value, 7);

        cursor.set(Cursor::At(Time::from_nanos(50)));
        assert_eq!(cache.latest().unwrap().value, 7);
        emit(1, None);
        settle().await;
        assert_eq!(cache.latest().unwrap().value, 7);
        emit(
            1,
            Some(Arc::new(Evaluation {
                state: State::Failed("request timed out".into()),
                ..(*result).clone()
            })),
        );
        settle().await;
        assert_eq!(cache.latest().unwrap().value, 7);
        assert_eq!(cache.status().message(), Some("request timed out"));
        emit(1, Some(Arc::clone(&result)));
        settle().await;
        let first = cache.latest().unwrap();
        let decoded_before_switch = calls.load(Ordering::Relaxed);
        // Also cover a revision change coalesced with the ready result.
        cursor.set(Cursor::At(Time::from_nanos(60)));
        emit(2, Some(result.clone()));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(calls.load(Ordering::Relaxed), decoded_before_switch);
        let mut replaced = (*result).clone();
        let State::Ready(Value::Records(samples)) = &mut replaced.state else {
            unreachable!()
        };
        Arc::make_mut(&mut samples[0].record).publication = Some(([1; 16], 7));
        emit(2, Some(Arc::new(replaced.clone())));
        settle().await;
        assert_eq!(
            calls.load(Ordering::Relaxed),
            decoded_before_switch + 1,
            "reused IDs from a new publisher must decode again"
        );
        assert!(!Arc::ptr_eq(&first, &cache.latest().unwrap()));
        let State::Ready(Value::Records(samples)) = &mut replaced.state else {
            unreachable!()
        };
        samples[0].topic.type_info = u32::type_info();
        emit(2, Some(Arc::new(replaced)));
        settle().await;
        assert_eq!(
            calls.load(Ordering::Relaxed),
            decoded_before_switch + 2,
            "schema changes invalidate the decode cache"
        );
    }
}
