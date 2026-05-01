use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use color_eyre::eyre::{Result, eyre};
use mcap::{Message, MessageStream, Summary};
use ros_z::context::{Context, ContextBuilder};
use ros_z_msgs::std_msgs::String as RosString;
use ros_z_record::{RecorderOptions, RecordingHandle, RecordingPlan, RecordingReport};
use tokio::{task::JoinHandle, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use zenoh::{Wait, config::WhatAmI};

static UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn records_single_topic_to_mcap() -> Result<()> {
    let router = TestRouter::new();
    let publisher_ctx = create_context(router.endpoint()).await?;
    let recorder_ctx = create_context(router.endpoint()).await?;
    let topic = unique_topic("/record_one");

    let publisher_node = create_node(&publisher_ctx, "publisher_one", false).await?;
    let publisher = boxed(publisher_node.publisher::<RosString>(&topic).build().await)?;
    let publisher_task = spawn_string_publisher(publisher, "hello");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let recorder_node = create_node(&recorder_ctx, "recorder_one", false).await?;
    let tempdir = tempfile::tempdir()?;
    let output = tempdir.path().join("single.mcap");
    let recording = start_recorder(
        Arc::clone(&recorder_node),
        output.clone(),
        vec![topic.clone()],
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(600)).await;
    stop_publisher(publisher_task).await?;
    let report = stop_recording(recording).await?;

    assert!(report.total_messages >= 2);
    assert!(report.silent_topics.is_empty());

    let bytes = fs::read(&output)?;
    let summary = Summary::read(&bytes)?.ok_or_else(|| eyre!("missing mcap summary"))?;
    let messages = collect_messages(&bytes)?;

    assert_eq!(summary.schemas.len(), 1);
    assert_eq!(summary.channels.len(), 1);
    assert!(!summary.chunk_indexes.is_empty());
    assert!(summary.stats.is_some());
    assert_eq!(
        summary.stats.as_ref().map(|stats| stats.message_count),
        Some(report.total_messages)
    );
    assert_eq!(messages.len() as u64, report.total_messages);
    assert_eq!(messages[0].channel.topic, topic);
    assert_eq!(messages[0].channel.message_encoding, "cdr");
    assert_eq!(
        messages[0]
            .channel
            .schema
            .as_ref()
            .map(|schema| schema.encoding.clone()),
        Some("ros-z/schema+json;v=2".to_string())
    );
    assert!(
        messages[0]
            .channel
            .metadata
            .get("schema_hash")
            .is_some_and(|schema_hash| schema_hash.starts_with("RZHS01_"))
    );
    assert!(
        messages[0]
            .channel
            .metadata
            .get("source_id")
            .is_some_and(|source_id| source_id.starts_with("gid:"))
    );
    assert!(
        messages
            .windows(2)
            .all(|pair| pair[1].sequence > pair[0].sequence)
    );
    assert!(messages.iter().all(|message| message.publish_time > 0));
    assert!(messages.iter().all(|message| message.log_time > 0));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn recorder_prepare_fails_when_target_topic_has_no_schema_service_source() -> Result<()> {
    let router = TestRouter::new();
    let recorder_ctx = create_context(router.endpoint()).await?;
    let recorder_node = create_node(&recorder_ctx, "recorder_missing_schema", true).await?;
    let tempdir = tempfile::tempdir()?;
    let output = tempdir.path().join("missing_schema.mcap");

    let prepared = RecordingPlan::build(
        recorder_node,
        RecorderOptions {
            output,
            topics: vec![unique_topic("/missing_schema")],
            discovery_timeout: Duration::from_millis(250),
            duration_limit: None,
            stats_interval: Duration::from_secs(1),
            session_metadata: BTreeMap::new(),
        },
    )
    .await;

    assert!(prepared.is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn records_two_publishers_as_two_channels() -> Result<()> {
    let router = TestRouter::new();
    let publisher_ctx_one = create_context(router.endpoint()).await?;
    let publisher_ctx_two = create_context(router.endpoint()).await?;
    let recorder_ctx = create_context(router.endpoint()).await?;
    let topic = unique_topic("/record_two_publishers");

    let publisher_node_one = create_node(&publisher_ctx_one, "publisher_one", false).await?;
    let publisher_node_two = create_node(&publisher_ctx_two, "publisher_two", false).await?;
    let publisher_one = boxed(
        publisher_node_one
            .publisher::<RosString>(&topic)
            .build()
            .await,
    )?;
    let publisher_two = boxed(
        publisher_node_two
            .publisher::<RosString>(&topic)
            .build()
            .await,
    )?;
    let publisher_task_one = spawn_string_publisher(publisher_one, "one");
    let publisher_task_two = spawn_string_publisher(publisher_two, "two");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let recorder_node = create_node(&recorder_ctx, "recorder_two_publishers", false).await?;
    let tempdir = tempfile::tempdir()?;
    let output = tempdir.path().join("two_publishers.mcap");
    let recording = start_recorder(
        Arc::clone(&recorder_node),
        output.clone(),
        vec![topic.clone()],
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(600)).await;
    stop_publisher(publisher_task_one).await?;
    stop_publisher(publisher_task_two).await?;
    let report = stop_recording(recording).await?;
    assert!(report.total_messages >= 2);

    let bytes = fs::read(&output)?;
    let summary = Summary::read(&bytes)?.ok_or_else(|| eyre!("missing mcap summary"))?;
    let source_ids: HashSet<_> = summary
        .channels
        .values()
        .map(|channel| channel.metadata.get("source_id").cloned())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| eyre!("missing source_id metadata"))?
        .into_iter()
        .collect();

    assert_eq!(summary.channels.len(), 2);
    assert_eq!(source_ids.len(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn cancellation_does_not_hang_on_silent_topics() -> Result<()> {
    let router = TestRouter::new();
    let publisher_ctx = create_context(router.endpoint()).await?;
    let recorder_ctx = create_context(router.endpoint()).await?;
    let topic = unique_topic("/record_silent");

    let publisher_node = create_node(&publisher_ctx, "publisher_silent", false).await?;
    let publisher = boxed(publisher_node.publisher::<RosString>(&topic).build().await)?;
    let publisher_task = spawn_string_publisher(publisher, "warmup");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let recorder_node = create_node(&recorder_ctx, "recorder_silent", false).await?;
    let tempdir = tempfile::tempdir()?;
    let prepared = prepare_recorder(
        Arc::clone(&recorder_node),
        tempdir.path().join("silent.mcap"),
        vec![topic.clone()],
    )
    .await?;
    stop_publisher(publisher_task).await?;

    let shutdown = CancellationToken::new();
    let handle = prepared.spawn(shutdown.clone()).await?;
    let report = stop_recording((shutdown, handle)).await?;

    assert_eq!(report.total_messages, 0);
    assert_eq!(report.silent_topics, vec![topic]);
    Ok(())
}

async fn create_node(
    context: &Context,
    prefix: &str,
    without_schema_service: bool,
) -> Result<Arc<ros_z::node::Node>> {
    let name = unique_name(prefix);
    let builder = context.create_node(&name);
    let builder = if without_schema_service {
        builder.without_schema_service()
    } else {
        builder
    };

    Ok(Arc::new(boxed(builder.build().await)?))
}

async fn create_context(endpoint: &str) -> Result<Context> {
    boxed(
        ContextBuilder::default()
            .disable_multicast_scouting()
            .with_connect_endpoints([endpoint])
            .with_logging_enabled()
            .build()
            .await,
    )
}

fn spawn_string_publisher(
    publisher: ros_z::pubsub::Publisher<RosString>,
    label: &'static str,
) -> (CancellationToken, JoinHandle<Result<()>>) {
    let shutdown = CancellationToken::new();
    let task_shutdown = shutdown.clone();
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut index = 0u64;

        loop {
            tokio::select! {
                _ = task_shutdown.cancelled() => break,
                _ = interval.tick() => {
                    boxed(publisher.publish(&RosString {
                        data: format!("{label}-{index}"),
                    }).await)?;
                    index += 1;
                }
            }
        }

        Ok(())
    });

    (shutdown, handle)
}

async fn stop_publisher(
    (shutdown, handle): (CancellationToken, JoinHandle<Result<()>>),
) -> Result<()> {
    shutdown.cancel();
    handle.await.map_err(|error| eyre!(error.to_string()))??;
    Ok(())
}

async fn prepare_recorder(
    recorder_node: Arc<ros_z::node::Node>,
    output: PathBuf,
    topics: Vec<String>,
) -> Result<RecordingPlan> {
    RecordingPlan::build(
        recorder_node,
        RecorderOptions {
            output,
            topics,
            discovery_timeout: Duration::from_secs(15),
            duration_limit: None,
            stats_interval: Duration::from_secs(1),
            session_metadata: BTreeMap::new(),
        },
    )
    .await
}

async fn start_recorder(
    recorder_node: Arc<ros_z::node::Node>,
    output: PathBuf,
    topics: Vec<String>,
) -> Result<(CancellationToken, RecordingHandle)> {
    let prepared = prepare_recorder(recorder_node, output, topics).await?;
    let shutdown = CancellationToken::new();
    let handle = prepared.spawn(shutdown.clone()).await?;
    Ok((shutdown, handle))
}

async fn stop_recording(
    (shutdown, handle): (CancellationToken, RecordingHandle),
) -> Result<RecordingReport> {
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(3), handle.wait())
        .await
        .map_err(|_| eyre!("recorder shutdown timed out"))?
}

fn collect_messages(bytes: &[u8]) -> Result<Vec<Message<'static>>> {
    Ok(MessageStream::new(bytes)?.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn unique_topic(prefix: &str) -> String {
    format!("{}-{}", prefix, UNIQUE_ID.fetch_add(1, Ordering::Relaxed))
}

fn unique_name(prefix: &str) -> String {
    format!("{}_{}", prefix, UNIQUE_ID.fetch_add(1, Ordering::Relaxed))
}

fn boxed<T>(result: std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>) -> Result<T> {
    result.map_err(|error| eyre!(error.to_string()))
}

struct TestRouter {
    endpoint: String,
    _session: zenoh::Session,
}

impl TestRouter {
    fn new() -> Self {
        for _attempt in 0..5u32 {
            let port = {
                let listener = std::net::TcpListener::bind("127.0.0.1:0")
                    .expect("failed to bind ephemeral port");
                listener.local_addr().expect("listener addr").port()
            };
            let endpoint = format!("tcp/127.0.0.1:{port}");

            let mut config = zenoh::Config::default();
            config.set_mode(Some(WhatAmI::Router)).unwrap();
            config
                .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
                .unwrap();
            config
                .insert_json5("scouting/multicast/enabled", "false")
                .unwrap();

            if let Ok(session) = zenoh::open(config).wait() {
                thread::sleep(Duration::from_millis(500));
                return Self {
                    endpoint,
                    _session: session,
                };
            }
        }

        panic!("failed to open test router after retries");
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
