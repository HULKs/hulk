use std::{collections::HashMap, num::NonZeroUsize, sync::Arc, time::Duration};

use ros_z::{
    Message,
    dynamic::{DynamicCdrCodec, DynamicPayload},
    message::WireDecoder,
};

use super::plan::{Sample, Selection, Source, State, TopicSource, Value};
use super::*;
use crate::{
    CachedSubscription, CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot,
    ObservationPolicy, SampleMetadata, SampleRecord, TopicReference,
    cache::CachedSubscriptionState,
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
    if topic.type_info != T::type_info() {
        return Err(query_error(format!(
            "{}: recorded schema does not match {}",
            topic.name,
            T::type_name()
        )));
    }
    let value = T::Codec::deserialize(&record.data).map_err(query_error)?;
    wrap_record(record, topic, value)
}

fn wrap_record<T>(record: &RawRecord, topic: &TopicInfo, value: T) -> Result<Arc<SampleRecord<T>>> {
    let (publisher, sequence) = record.publication.unwrap_or(([0; 16], record.id as i64));
    Ok(Arc::new(SampleRecord {
        value,
        source_time: record.source_time,
        effective_time: record.time,
        transport_time: record.log_time,
        publication_id: ros_z::pubsub::Received {
            message: (),
            transport_time: record.log_time,
            source_time: record.source_time,
            sequence_number: sequence,
            source_global_id: publisher.into(),
        }
        .publication_id(),
        metadata: Arc::new(SampleMetadata {
            topic_reference: TopicReference::new(&topic.name)?,
            resolved_topic: topic.name.clone(),
            type_info: topic.type_info.clone(),
        }),
    }))
}

pub(crate) fn typed_cache<T: Message>(
    connection: &Connection,
    topic: &str,
    policy: ObservationPolicy,
    projection: TimeProjection,
) -> Result<CachedSubscription<T>> {
    cache(connection, topic, policy, projection, decode_record::<T>)
}

pub(crate) fn dynamic_cache(
    connection: &Connection,
    topic: &str,
    policy: ObservationPolicy,
    projection: TimeProjection,
) -> Result<CachedSubscription<DynamicPayload>> {
    cache(connection, topic, policy, projection, |record, topic| {
        let schema = topic
            .schema
            .as_ref()
            .ok_or_else(|| query_error("topic schema unavailable"))?;
        let payload = DynamicCdrCodec::decode(&record.data, schema).map_err(query_error)?;
        wrap_record(record, topic, payload)
    })
}

fn cache<T: Send + Sync + 'static>(
    connection: &Connection,
    topic: &str,
    policy: ObservationPolicy,
    projection: TimeProjection,
    decode: impl Fn(&RawRecord, &TopicInfo) -> Result<Arc<SampleRecord<T>>> + Send + 'static,
) -> Result<CachedSubscription<T>> {
    let selection = match policy.retention() {
        crate::RetentionPolicy::LatestOnly => Selection::latest(),
        crate::RetentionPolicy::TimeWindow(window) if window.duration() == Duration::MAX => {
            Selection::AtOrBefore {
                count: window
                    .max_samples()
                    .unwrap_or(NonZeroUsize::new(MAX_ROWS).expect("nonzero response limit")),
            }
        }
        crate::RetentionPolicy::TimeWindow(window) => Selection::Window {
            before: window.duration(),
            after: Duration::ZERO,
        },
    };
    let source = Source::from(TopicSource {
        topic: topic.into(),
        live_policy: policy,
        projection,
    });
    let raw = connection.backend.observe(
        connection.cursor.clone(),
        source.select(selection, plan::cursor()).plan(),
    )?;
    let mut updates = raw.subscribe_updates();
    let mut positions = connection.cursor.subscribe();
    let state = Arc::new(CachedSubscriptionState::new(
        CachedSubscriptionStatusSnapshot::with_metadata(
            CachedSubscriptionStatus::WaitingForFirstSample,
            topic.into(),
            ros_z::TypeInfo::new("pending", ros_z::SchemaHash::zero()),
        ),
        policy.retention(),
        policy.update_buffer_capacity(),
    ));
    state.attach_query(raw.clone());
    state.spawn_receive(move |state, cancellation| async move {
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
                    state.set_receive_error(CachedSubscriptionStatus::decode_error(error.clone()));
                } else if let State::Ready(Value::Records(samples)) = &snapshot.evaluation.state {
                    let sample_type = samples.first().map(|sample| &sample.topic.type_info);
                    if sample_type.is_some() && decoded_type.as_ref() != sample_type {
                        decoded.clear();
                        decoded_type = sample_type.cloned();
                        applied = None;
                    }
                    let limit = match policy.retention() {
                        crate::RetentionPolicy::TimeWindow(window) => {
                            window.max_samples().map_or(usize::MAX, NonZeroUsize::get)
                        }
                        crate::RetentionPolicy::LatestOnly => 1,
                    };
                    let samples = &samples[samples.len().saturating_sub(limit)..];
                    let unchanged = applied.as_ref().is_some_and(|(keys, revision)| {
                        *revision == snapshot.revision
                            && keys.iter().copied().eq(samples.iter().map(record_key))
                    });
                    if !unchanged {
                        let records = decode_samples(&mut decoded, samples, |sample| {
                            decode(&sample.record, &sample.topic).map_err(|error| error.to_string())
                        });
                        match records {
                            Ok(records) => {
                                state.replace_records(records, snapshot.revision);
                            }
                            Err(error) => state
                                .set_receive_error(CachedSubscriptionStatus::decode_error(error)),
                        }
                        applied =
                            Some((samples.iter().map(record_key).collect(), snapshot.revision));
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
    });
    Ok(state.handle())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use parking_lot::Mutex;
    use tokio::sync::oneshot;

    use super::*;
    use crate::query::plan::Evaluation;

    #[test]
    fn decoding_visits_records_after_errors_and_reuses_successes_and_failures() {
        let samples: Vec<_> = (1..=3)
            .map(|id| Sample {
                topic: TopicInfo {
                    name: "/test".into(),
                    type_info: u64::type_info(),
                    schema: None,
                    error: None,
                },
                record: Arc::new(RawRecord {
                    id,
                    publication: None,
                    time: Time::from_nanos(id as i64),
                    source_time: Time::zero(),
                    log_time: None,
                    data: Arc::from([]),
                }),
            })
            .collect();
        let mut decoded = HashMap::new();
        let mut calls = Vec::new();
        for _ in 0..2 {
            assert!(
                decode_samples(&mut decoded, &samples, |sample| {
                    calls.push(sample.record.id);
                    if sample.record.id == 2 {
                        Err("invalid payload".into())
                    } else {
                        wrap_record(&sample.record, &sample.topic, sample.record.id)
                            .map_err(|e| e.to_string())
                    }
                })
                .is_err()
            );
        }
        assert_eq!(calls, vec![1, 2, 3]);
        let good = decoded[&record_key(&samples[2])].as_ref().unwrap().clone();
        let result = decode_samples(&mut decoded, &samples[2..], |_| {
            panic!("retained record decoded again")
        })
        .unwrap();
        assert!(Arc::ptr_eq(&good, &result[0]));
        assert_eq!(decoded.len(), 1);
    }

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

    impl Backend for TestBackend {
        fn topics(&self) -> TopicList {
            TopicList(watch::channel(Arc::new(TopicSnapshot::default())).1)
        }

        fn observe(&self, _: CursorHandle, _: plan::QueryPlan) -> Result<PlanObservation> {
            Ok(self.0.clone())
        }
    }

    async fn settle() {
        // Let the worker consume watch notifications before checking for silence.
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seek_during_first_decode_rejects_the_obsolete_installation() {
        let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(100)));
        let (raw, sender) = PlanObservation::channel(cursor.clone());
        let result = ready_result();
        sender.send_replace(Arc::new(PlanSnapshot {
            revision: 0,
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
            "/test",
            ObservationPolicy::default(),
            TimeProjection::SourceTime,
            move |record, topic| {
                if let Some(started) = started.lock().take() {
                    started.send(()).unwrap();
                    gate.lock().recv_timeout(Duration::from_secs(5)).unwrap();
                }
                wrap_record(record, topic, record.id)
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
        cursor.set(Cursor::At(Time::from_nanos(50)));
        sender.send_replace(Arc::new(PlanSnapshot {
            revision: 1,
            evaluation: Arc::new(Evaluation {
                state: State::Pending,
                ..(*result).clone()
            }),
        }));
        release.send(()).unwrap();
        settle().await;
        assert!(cache.latest().is_none());
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[tokio::test]
    async fn pending_queries_retain_results_until_a_definitive_replacement() {
        let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(100)));
        let (raw, sender) = PlanObservation::channel(cursor.clone());
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let cache = cache(
            &Connection {
                backend: Arc::new(TestBackend(raw)),
                cursor: cursor.clone(),
            },
            "/test",
            ObservationPolicy::time_window(Duration::from_secs(1)).unwrap(),
            TimeProjection::default(),
            move |record, topic| {
                counter.fetch_add(1, Ordering::Relaxed);
                if record.id == 8 {
                    return Err(query_error("bad row"));
                }
                wrap_record(record, topic, record.id)
            },
        )
        .unwrap();
        let result = ready_result();
        let emit = |revision, value: Option<Arc<Evaluation>>| {
            sender.send_replace(Arc::new(PlanSnapshot {
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
        for tick in 101..105 {
            cursor.advance(Time::from_nanos(tick));
            settle().await;
            assert!(matches!(updates.try_recv(), Ok(None)));
            assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        let mut failing = (*result).clone();
        let State::Ready(Value::Records(samples)) = &mut failing.state else {
            unreachable!()
        };
        let mut bad = (*samples[0].record).clone();
        bad.id = 8;
        samples.push(Sample {
            record: Arc::new(bad),
            topic: samples[0].topic.clone(),
        });
        let failing = Arc::new(failing);
        emit(0, Some(Arc::clone(&failing)));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(cache.status().message(), Some("query: bad row"));
        let mut errors = cache.subscribe_updates().unwrap();
        cursor.advance(Time::from_nanos(105));
        settle().await;
        assert!(matches!(errors.try_recv(), Ok(None)));
        // A fresh result Arc must reuse failed decodes too.
        emit(0, Some(Arc::new((*failing).clone())));
        settle().await;
        assert!(matches!(errors.try_recv(), Ok(None)));
        emit(0, Some(Arc::clone(&result)));
        settle().await;
        assert_eq!(cache.status().status(), &CachedSubscriptionStatus::Ready);
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(calls.load(Ordering::Relaxed), 2);

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
        provisional.dependencies.push(plan::Dependency {
            source: TopicSource {
                topic: "/test".into(),
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
        // Also cover a revision change coalesced with the ready result.
        cursor.set(Cursor::At(Time::from_nanos(60)));
        emit(2, Some(result.clone()));
        settle().await;
        assert!(Arc::ptr_eq(&first, &cache.latest().unwrap()));
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        let mut replaced = (*result).clone();
        let State::Ready(Value::Records(samples)) = &mut replaced.state else {
            unreachable!()
        };
        Arc::make_mut(&mut samples[0].record).publication = Some(([1; 16], 7));
        emit(2, Some(Arc::new(replaced.clone())));
        settle().await;
        assert_eq!(
            calls.load(Ordering::Relaxed),
            4,
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
            5,
            "schema changes invalidate the decode cache"
        );
    }
}
