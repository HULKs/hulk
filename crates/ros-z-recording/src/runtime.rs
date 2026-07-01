use std::fs::OpenOptions;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use ros_z::graph::Graph;
use ros_z::node::Node;
use tokio::sync::{mpsc, watch};

use crate::sample::{RecordedSample, sample_to_record};
use crate::summary::{RecordingSummary, TopicSummary};
use crate::topic::{ResolvedTopic, resolve_topics};
use crate::writer::{McapWriterSink, WriterTopicSummary};
use crate::{RecordingConfig, RecordingError, Result};

pub struct RecordingSession {
    output_path: PathBuf,
    start_time: SystemTime,
    topics: Vec<ResolvedTopic>,
    drops: Vec<Arc<AtomicU64>>,
    stop: watch::Sender<bool>,
    failure_rx: mpsc::Receiver<()>,
    receive_tasks: Vec<(String, tokio::task::JoinHandle<Result<()>>)>,
    writer_task: tokio::task::JoinHandle<Result<Vec<WriterTopicSummary>>>,
}

impl RecordingSession {
    pub async fn start(node: Arc<Node>, graph: &Graph, config: RecordingConfig) -> Result<Self> {
        let (output_path, requested_topics, writer_queue_capacity) = config.into_parts();

        if requested_topics.is_empty() {
            return Err(RecordingError::EmptyTopicSelection);
        }

        if output_path.exists() {
            return Err(RecordingError::OutputAlreadyExists { path: output_path });
        }

        let topics = resolve_topics(Arc::clone(&node), graph, &requested_topics).await?;

        let mut subscribers = Vec::with_capacity(topics.len());
        for (topic_index, topic) in topics.iter().enumerate() {
            let subscriber = node
                .dynamic_subscriber(topic.topic(), topic.type_info(), Arc::clone(topic.schema()))
                .raw()
                .build()
                .await
                .map_err(|source| RecordingError::Subscribe {
                    topic: topic.topic().to_string(),
                    source,
                })?;
            subscribers.push((topic_index, topic.topic().to_string(), subscriber));
        }

        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .map_err(|source| RecordingError::OutputCreate {
                path: output_path.clone(),
                source,
            })?;
        let mut sink = McapWriterSink::new(BufWriter::new(output), &topics, &requested_topics)?;
        let (sample_tx, mut sample_rx) =
            mpsc::channel::<RecordedSample>(writer_queue_capacity.get());
        let (stop_tx, stop_rx) = watch::channel(false);
        let (failure_tx, failure_rx) = mpsc::channel::<()>(1);
        let drops = (0..topics.len())
            .map(|_| Arc::new(AtomicU64::new(0)))
            .collect::<Vec<_>>();

        let writer_task = spawn_writer_task(failure_tx.clone(), move || {
            while let Some(sample) = sample_rx.blocking_recv() {
                sink.write_sample(&sample)?;
            }
            sink.finish()
        });

        let mut receive_tasks = Vec::with_capacity(topics.len());
        for (topic_index, task_topic, mut subscriber) in subscribers {
            let receive_topic = task_topic.clone();
            let sender = sample_tx.clone();
            let mut stop_rx = stop_rx.clone();
            let stop_sender = stop_tx.clone();
            let failure_sender = failure_tx.clone();
            let drop_counter = Arc::clone(&drops[topic_index]);
            let handle = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        changed = stop_rx.changed() => {
                            if changed.is_err() || *stop_rx.borrow() {
                                return Ok(());
                            }
                        }
                        sample = subscriber.recv() => {
                            let sample = match sample {
                                Ok(sample) => sample,
                                Err(source) => {
                                    let _ = failure_sender.try_send(());
                                    let _ = stop_sender.send(true);
                                    return Err(RecordingError::Receive {
                                        topic: receive_topic.clone(),
                                        source,
                                    });
                                }
                            };
                            match enqueue_sample(topic_index, sample, &sender, &drop_counter, SystemTime::now()) {
                                Ok(EnqueueOutcome::Sent | EnqueueOutcome::DroppedFull) => {}
                                Ok(EnqueueOutcome::Closed) => return Ok(()),
                                Err(error) => {
                                    let _ = failure_sender.try_send(());
                                    let _ = stop_sender.send(true);
                                    return Err(error);
                                }
                            }
                        }
                    }
                }
            });
            receive_tasks.push((task_topic, handle));
        }
        drop(sample_tx);
        drop(failure_tx);

        Ok(Self {
            output_path,
            start_time: SystemTime::now(),
            topics,
            drops,
            stop: stop_tx,
            failure_rx,
            receive_tasks,
            writer_task,
        })
    }

    pub fn resolved_topics(&self) -> &[ResolvedTopic] {
        &self.topics
    }

    pub fn output_path(&self) -> &std::path::Path {
        &self.output_path
    }

    pub async fn wait_for_failure(&mut self) {
        let _ = self.failure_rx.recv().await;
    }

    pub async fn stop(self) -> Result<RecordingSummary> {
        let _ = self.stop.send(true);
        let mut receive_error = None;
        for (topic, task) in self.receive_tasks {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(source)) => {
                    if receive_error.is_none() {
                        receive_error = Some(RecordingError::ReceiveTask {
                            topic,
                            source: Box::new(source),
                        });
                    }
                }
                Err(source) => {
                    if receive_error.is_none() {
                        receive_error = Some(RecordingError::ReceiveTask {
                            topic,
                            source: Box::new(RecordingError::Join(source)),
                        });
                    }
                }
            }
        }

        let writer_counts = match self.writer_task.await {
            Ok(Ok(counts)) => counts,
            Ok(Err(finalize)) => {
                if let Some(receive) = receive_error {
                    return Err(RecordingError::ReceiveAndFinalize {
                        receive: Box::new(receive),
                        finalize: Box::new(finalize),
                    });
                }
                return Err(finalize);
            }
            Err(source) => {
                let finalize = RecordingError::Join(source);
                if let Some(receive) = receive_error {
                    return Err(RecordingError::ReceiveAndFinalize {
                        receive: Box::new(receive),
                        finalize: Box::new(finalize),
                    });
                }
                return Err(finalize);
            }
        };
        let end_time = SystemTime::now();
        let topic_summaries = self
            .topics
            .iter()
            .zip(self.drops.iter())
            .map(|(topic, drops)| {
                topic_summary_with_drops(
                    topic.topic(),
                    topic.type_name(),
                    topic.schema_hash(),
                    drops.load(Ordering::Relaxed),
                )
            })
            .collect();

        let summary = build_summary(
            self.output_path,
            self.start_time,
            end_time,
            topic_summaries,
            writer_counts,
        );

        if let Some(source) = receive_error {
            return Err(RecordingError::RecordingStoppedAfterReceiveError {
                source: Box::new(source),
                summary: Box::new(summary),
            });
        }

        Ok(summary)
    }
}

fn spawn_writer_task(
    failure_sender: mpsc::Sender<()>,
    write: impl FnOnce() -> Result<Vec<WriterTopicSummary>> + Send + 'static,
) -> tokio::task::JoinHandle<Result<Vec<WriterTopicSummary>>> {
    tokio::task::spawn_blocking(move || {
        let result = write();
        if result.is_err() {
            let _ = failure_sender.try_send(());
        }
        result
    })
}

enum EnqueueOutcome {
    Sent,
    DroppedFull,
    Closed,
}

fn enqueue_sample(
    topic_index: usize,
    sample: zenoh::sample::Sample,
    sender: &mpsc::Sender<RecordedSample>,
    drop_counter: &AtomicU64,
    receive_time: SystemTime,
) -> Result<EnqueueOutcome> {
    let permit = match sender.try_reserve() {
        Ok(permit) => permit,
        Err(mpsc::error::TrySendError::Full(())) => {
            drop_counter.fetch_add(1, Ordering::Relaxed);
            return Ok(EnqueueOutcome::DroppedFull);
        }
        Err(mpsc::error::TrySendError::Closed(())) => return Ok(EnqueueOutcome::Closed),
    };

    let recorded = sample_to_record(topic_index, sample, receive_time)?;
    permit.send(recorded);
    Ok(EnqueueOutcome::Sent)
}

pub(crate) fn topic_summary_with_drops(
    topic: &str,
    type_name: &str,
    schema_hash: &str,
    drops: u64,
) -> TopicSummary {
    TopicSummary {
        topic: topic.to_string(),
        type_name: type_name.to_string(),
        schema_hash: schema_hash.to_string(),
        messages: 0,
        bytes: 0,
        drops,
    }
}

pub(crate) fn build_summary(
    output_path: PathBuf,
    start_time: SystemTime,
    end_time: SystemTime,
    mut topics: Vec<TopicSummary>,
    writer_counts: Vec<WriterTopicSummary>,
) -> RecordingSummary {
    for (topic, counts) in topics.iter_mut().zip(writer_counts) {
        topic.messages = counts.messages;
        topic.bytes = counts.bytes;
    }

    RecordingSummary {
        output_path,
        start_time,
        end_time,
        topics,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, UNIX_EPOCH};

    use ros_z::EndpointGlobalId;
    use ros_z::attachment::Attachment;
    use tokio::sync::{mpsc, oneshot, watch};
    use tokio::time::timeout;

    use crate::runtime::{
        RecordingSession, build_summary, enqueue_sample, spawn_writer_task,
        topic_summary_with_drops,
    };
    use crate::sample::RecordedSample;
    use crate::summary::TopicSummary;
    use crate::writer::WriterTopicSummary;
    use crate::{RecordingConfig, RecordingError};

    #[test]
    fn empty_topic_selection_is_rejected_before_runtime_start() {
        let temp = tempfile::tempdir().expect("tempdir creates");

        match RecordingConfig::new(temp.path().join("empty-topics.mcap"), Vec::new()) {
            Err(RecordingError::EmptyTopicSelection) => {}
            Err(error) => panic!("expected EmptyTopicSelection, got {error:?}"),
            Ok(_) => panic!("empty topic selection must fail before starting a session"),
        }
    }

    #[tokio::test]
    async fn enqueue_sample_drops_full_queue_without_converting_payload() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender
            .try_send(RecordedSample {
                topic_index: 0,
                sequence: 1,
                log_time: 1,
                publish_time: 1,
                payload: b"queued".to_vec(),
            })
            .expect("fill queue");
        let drops = Arc::new(AtomicU64::new(0));
        let sample = sample_without_attachment("would fail if converted");

        enqueue_sample(0, sample, &sender, &drops, UNIX_EPOCH)
            .expect("full queue should drop before sample conversion");

        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(
            receiver
                .recv()
                .await
                .expect("queued sample remains")
                .payload,
            b"queued"
        );
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn wait_for_failure_wakes_when_writer_task_fails() {
        let (stop_tx, _stop_rx) = watch::channel(false);
        let (failure_tx, failure_rx) = mpsc::channel(1);
        let writer_task =
            spawn_writer_task(failure_tx, || Err(RecordingError::EmptyTopicSelection));
        let mut session = RecordingSession {
            output_path: PathBuf::from("writer-error.mcap"),
            start_time: UNIX_EPOCH,
            topics: Vec::new(),
            drops: Vec::new(),
            stop: stop_tx,
            failure_rx,
            receive_tasks: Vec::new(),
            writer_task,
        };

        timeout(Duration::from_secs(1), session.wait_for_failure())
            .await
            .expect("writer failure should notify waiters");

        let error = session
            .stop()
            .await
            .expect_err("writer failure should return");
        assert!(matches!(error, RecordingError::EmptyTopicSelection));
    }

    #[tokio::test]
    async fn stop_joins_remaining_receivers_and_finalizes_writer_after_receive_join_error() {
        let (stop_tx, _stop_rx) = watch::channel(false);
        let (_failure_tx, failure_rx) = mpsc::channel(1);
        let (second_receive_release_tx, second_receive_release_rx) = oneshot::channel();
        let (writer_release_tx, writer_release_rx) = oneshot::channel();
        let (writer_finalized_tx, writer_finalized_rx) = oneshot::channel();
        let panicking_receive_task = tokio::spawn(async move {
            panic!("receive task join failure");
            #[allow(unreachable_code)]
            Ok::<(), RecordingError>(())
        });
        let blocked_receive_task = tokio::spawn(async move {
            let _ = second_receive_release_rx.await;
            Ok::<(), RecordingError>(())
        });
        let writer_task = tokio::spawn(async move {
            let _ = writer_release_rx.await;
            let _ = writer_finalized_tx.send(());
            Ok::<Vec<WriterTopicSummary>, RecordingError>(Vec::new())
        });
        let session = RecordingSession {
            output_path: PathBuf::from("join-error.mcap"),
            start_time: UNIX_EPOCH,
            topics: Vec::new(),
            drops: Vec::new(),
            stop: stop_tx,
            failure_rx,
            receive_tasks: vec![
                ("/panics".to_string(), panicking_receive_task),
                ("/blocked".to_string(), blocked_receive_task),
            ],
            writer_task,
        };
        let mut stop_task = tokio::spawn(session.stop());

        assert!(
            timeout(Duration::from_millis(50), &mut stop_task)
                .await
                .is_err(),
            "stop returned before joining all receive tasks"
        );

        second_receive_release_tx
            .send(())
            .expect("blocked receive task should still be pending");
        assert!(
            timeout(Duration::from_millis(50), &mut stop_task)
                .await
                .is_err(),
            "stop returned before awaiting writer finalization"
        );

        writer_release_tx
            .send(())
            .expect("writer task should still be pending");
        let result = timeout(Duration::from_secs(1), stop_task)
            .await
            .expect("stop should finish after writer finalizes")
            .expect("stop task should not panic");
        writer_finalized_rx
            .await
            .expect("writer finalization should be observed");

        let Err(RecordingError::RecordingStoppedAfterReceiveError { source, .. }) = result else {
            panic!("expected receive task join failure after finalization, got {result:?}");
        };
        match *source {
            RecordingError::ReceiveTask { topic, source } => {
                assert_eq!(topic, "/panics");
                assert!(matches!(*source, RecordingError::Join(_)));
            }
            error => panic!("expected ReceiveTask source, got {error:?}"),
        }
    }

    #[test]
    fn build_summary_combines_writer_counts_and_drop_counts() {
        let topics = vec![topic_summary_with_drops(
            "/demo",
            "test_msgs::Demo",
            "RZHS02_demo",
            3,
        )];
        let summary = build_summary(
            PathBuf::from("demo.mcap"),
            UNIX_EPOCH,
            UNIX_EPOCH + Duration::from_secs(2),
            topics,
            vec![WriterTopicSummary {
                messages: 4,
                bytes: 20,
            }],
        );

        assert_eq!(summary.output_path, PathBuf::from("demo.mcap"));
        assert_eq!(summary.duration(), Duration::from_secs(2));
        assert_eq!(summary.topics[0].messages, 4);
        assert_eq!(summary.topics[0].bytes, 20);
        assert_eq!(summary.topics[0].drops, 3);
    }

    #[test]
    fn topic_summary_with_drops_initializes_counts_from_resolved_metadata() {
        let summary: TopicSummary =
            topic_summary_with_drops("/demo", "test_msgs::Demo", "RZHS02_demo", 9);

        assert_eq!(summary.topic, "/demo");
        assert_eq!(summary.type_name, "test_msgs::Demo");
        assert_eq!(summary.schema_hash, "RZHS02_demo");
        assert_eq!(summary.messages, 0);
        assert_eq!(summary.bytes, 0);
        assert_eq!(summary.drops, 9);
    }

    fn sample_without_attachment(payload: &str) -> zenoh::sample::Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        zenoh::sample::SampleBuilder::put(key_expr, payload).into()
    }

    #[allow(dead_code)]
    fn sample_with_attachment(sequence: i64, payload: &str) -> zenoh::sample::Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let attachment = Attachment::with_source_time(
            sequence,
            EndpointGlobalId::from([7; 16]),
            ros_z::time::Time::from_nanos(123_456),
        );
        zenoh::sample::SampleBuilder::put(key_expr, payload)
            .attachment(attachment)
            .into()
    }
}
