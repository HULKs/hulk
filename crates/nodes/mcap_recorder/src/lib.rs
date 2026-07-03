use std::{
    boxed::Box, collections::BTreeMap, fs::File, future::Future, io::BufWriter, num::NonZeroUsize,
    path::PathBuf, pin::Pin, sync::Arc, time::Duration,
};

use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use mcap::{Compression, WriteOptions, Writer, records::MessageHeader};
use ros_z::{
    Message,
    attachment::Attachment,
    dynamic::DiscoveredTopicSchema,
    prelude::*,
    pubsub::RawSubscriber,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use zenoh::sample::Sample;

type ChannelId = u16;
type RecorderTasks = FuturesUnordered<BoxFuture<'static, Result<RecordedSample>>>;
const RAW_IMAGE_TOPIC: &str = "inputs/stereo_image_pair";

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct McapRecorderParameters {
    pub enable: bool,
    pub max_duration: Option<Duration>,
    pub include_raw_images: bool,
    pub raw_image_min_interval: Option<Duration>,
    pub queue_depth: usize,
    pub schema_discovery_timeout: Duration,
    pub topics: Vec<String>,
}

pub fn run_boxed(
    ctx: Arc<Context>,
    log_path: Option<PathBuf>,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx, log_path))
}

async fn run(ctx: Arc<Context>, log_path: Option<PathBuf>) -> Result<()> {
    let Some(log_path) = log_path else {
        return Ok(());
    };
    let mcap_path = log_path.join("recording.mcap");

    let node = ctx.create_node("mcap_recorder").build().await?;
    let parameters = node.bind_parameter_as::<McapRecorderParameters>("mcap_recorder")?;
    let parameters = parameters.snapshot().typed().clone();

    if !parameters.enable {
        std::future::pending::<()>().await;
        return Ok(());
    }

    tokio::fs::create_dir_all(&log_path)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to create recorder output directory {}",
                log_path.display()
            )
        })?;

    let write_queue_depth = NonZeroUsize::new(parameters.queue_depth)
        .unwrap_or(NonZeroUsize::MIN)
        .get();
    let (write_sender, write_receiver) = mpsc::channel(write_queue_depth);
    let writer_task = spawn_writer_task(
        mcap_path.clone(),
        parameters.raw_image_min_interval,
        write_receiver,
    );

    let mut recorders = RecorderTasks::new();
    for topic in &parameters.topics {
        subscribe_topic(
            &node,
            &mut recorders,
            parameters.queue_depth,
            parameters.schema_discovery_timeout,
            topic,
        )
        .await?;
    }

    if parameters.include_raw_images {
        subscribe_topic(
            &node,
            &mut recorders,
            parameters.queue_depth,
            parameters.schema_discovery_timeout,
            RAW_IMAGE_TOPIC,
        )
        .await?;
    }

    tracing::info!(
        path = %mcap_path.display(),
        include_raw_images = parameters.include_raw_images,
        raw_image_min_interval = ?parameters.raw_image_min_interval,
        queue_depth = parameters.queue_depth,
        topics = parameters.topics.len(),
        compression = "lz4",
        "localization recording started"
    );

    let recorder_result =
        record_samples(&mut recorders, parameters.max_duration, &write_sender).await;
    drop(write_sender);

    let writer_result = writer_task
        .await
        .wrap_err("localization recording writer task failed to join")?;
    let samples_written = writer_result.as_ref().copied().unwrap_or_default();
    tracing::info!(
        path = %mcap_path.display(),
        samples_written,
        "localization recording finished"
    );

    writer_result?;
    recorder_result?;

    Ok(())
}

fn spawn_writer_task(
    mcap_path: PathBuf,
    raw_image_min_interval: Option<Duration>,
    write_receiver: mpsc::Receiver<WriteRequest>,
) -> tokio::task::JoinHandle<Result<usize>> {
    tokio::task::spawn_blocking(move || {
        write_samples(mcap_path, raw_image_min_interval, write_receiver)
    })
}

fn write_samples(
    mcap_path: PathBuf,
    raw_image_min_interval: Option<Duration>,
    mut write_receiver: mpsc::Receiver<WriteRequest>,
) -> Result<usize> {
    let file = File::create(&mcap_path).wrap_err_with(|| {
        format!(
            "failed to create localization recording {}",
            mcap_path.display()
        )
    })?;
    let mut writer = McapWriter::new(BufWriter::new(file), raw_image_min_interval)?;
    let mut samples_written = 0;

    while let Some(WriteRequest { channel, sample }) = write_receiver.blocking_recv() {
        if writer.write(&channel, &sample)? {
            samples_written += 1;
        }
    }

    writer.finish()?;

    Ok(samples_written)
}

async fn subscribe_topic(
    node: &Node,
    recorders: &mut RecorderTasks,
    queue_depth: usize,
    schema_discovery_timeout: Duration,
    topic: &str,
) -> Result<()> {
    let discovered = node
        .discover_topic_schema(topic, schema_discovery_timeout)
        .await
        .wrap_err_with(|| format!("failed to discover schema for {topic}"))?;
    let subscriber = node
        .dynamic_raw_subscriber(topic, discovered.type_info())
        .qos(recorder_qos(queue_depth))
        .build()
        .await
        .wrap_err_with(|| format!("failed to subscribe to {topic}"))?;
    let recorder = TopicRecorder {
        subscriber,
        channel: Arc::new(RecordedChannel::for_discovered(topic, &discovered)?),
    };
    tracing::info!(
        topic,
        qualified_topic = %discovered.qualified_topic,
        type_name = %discovered.root_name,
        schema_hash = %discovered.schema_hash.to_hash_string(),
        "localization recorder subscribed to topic"
    );

    recorders.push(receive_sample(recorder));

    Ok(())
}

fn recorder_qos(queue_depth: usize) -> QosProfile {
    QosProfile {
        reliability: QosReliability::BestEffort,
        history: QosHistory::KeepLast(NonZeroUsize::new(queue_depth).unwrap_or(NonZeroUsize::MIN)),
        ..Default::default()
    }
}

fn receive_sample(mut recorder: TopicRecorder) -> BoxFuture<'static, Result<RecordedSample>> {
    async move {
        let sample = recorder.subscriber.recv().await.wrap_err_with(|| {
            format!(
                "failed to receive raw sample from {}",
                recorder.channel.topic
            )
        })?;

        Ok(RecordedSample { recorder, sample })
    }
    .boxed()
}

async fn record_samples(
    recorders: &mut RecorderTasks,
    max_duration: Option<Duration>,
    write_sender: &mpsc::Sender<WriteRequest>,
) -> Result<()> {
    if let Some(max_duration) = max_duration {
        let timer = tokio::time::sleep(max_duration);
        tokio::pin!(timer);

        loop {
            tokio::select! {
                _ = &mut timer => return Ok(()),
                result = recorders.next() => {
                    if !handle_recorded_sample(result, recorders, write_sender).await? {
                        return Ok(());
                    }
                }
            }
        }
    }

    while let Some(result) = recorders.next().await {
        handle_recorded_sample(Some(result), recorders, write_sender).await?;
    }

    Ok(())
}

async fn handle_recorded_sample(
    result: Option<Result<RecordedSample>>,
    recorders: &mut RecorderTasks,
    write_sender: &mpsc::Sender<WriteRequest>,
) -> Result<bool> {
    let Some(result) = result else {
        return Ok(false);
    };
    let RecordedSample { recorder, sample } = result?;

    write_sender
        .send(WriteRequest {
            channel: Arc::clone(&recorder.channel),
            sample,
        })
        .await
        .map_err(|_| eyre!("localization recording writer task stopped before receiving sample"))?;
    recorders.push(receive_sample(recorder));

    Ok(true)
}

struct RecordedChannel {
    topic: String,
    schema_name: String,
    schema_data: Vec<u8>,
    metadata: BTreeMap<String, String>,
}

impl RecordedChannel {
    fn for_discovered(topic: &str, discovered: &DiscoveredTopicSchema) -> Result<Self> {
        let schema_name = discovered.root_name.clone();
        let schema_data = serde_json::to_vec(discovered.schema.as_ref())
            .wrap_err_with(|| format!("failed to serialize schema for {topic}"))?;
        let mut metadata = BTreeMap::new();
        metadata.insert("ros_z.type_name".to_string(), schema_name.clone());
        metadata.insert(
            "ros_z.schema_hash".to_string(),
            discovered.schema_hash.to_hash_string(),
        );

        Ok(Self {
            topic: topic.to_string(),
            schema_name,
            schema_data,
            metadata,
        })
    }
}

struct TopicRecorder {
    subscriber: RawSubscriber,
    channel: Arc<RecordedChannel>,
}

struct RecordedSample {
    recorder: TopicRecorder,
    sample: Sample,
}

struct WriteRequest {
    channel: Arc<RecordedChannel>,
    sample: Sample,
}

struct McapWriter<W: std::io::Write + std::io::Seek> {
    writer: Writer<W>,
    channel_mapping: BTreeMap<String, ChannelId>,
    last_written_source_time_by_topic: BTreeMap<String, Time>,
    raw_image_min_interval: Option<Duration>,
}

impl<W> McapWriter<W>
where
    W: std::io::Write + std::io::Seek,
{
    fn new(writer: W, raw_image_min_interval: Option<Duration>) -> Result<Self> {
        Ok(Self {
            writer: WriteOptions::new()
                .compression(Some(Compression::Lz4))
                .create(writer)?,
            channel_mapping: BTreeMap::new(),
            last_written_source_time_by_topic: BTreeMap::new(),
            raw_image_min_interval,
        })
    }

    fn write(&mut self, channel: &RecordedChannel, sample: &Sample) -> Result<bool> {
        let Some(raw_attachment) = sample.attachment() else {
            tracing::warn!(
                topic = channel.topic,
                "localization recorder skipped sample without ros-z attachment"
            );
            return Ok(false);
        };
        let attachment = match Attachment::try_from(raw_attachment) {
            Ok(attachment) => attachment,
            Err(error) => {
                tracing::warn!(
                    topic = channel.topic,
                    ?error,
                    "localization recorder skipped sample with invalid ros-z attachment"
                );
                return Ok(false);
            }
        };
        let source_time = attachment.source_time();
        if !self.should_write_sample(&channel.topic, source_time) {
            return Ok(false);
        }
        let transport_time = sample
            .timestamp()
            .map(|timestamp| Time::from_wallclock(timestamp.get_time().to_system_time()))
            .unwrap_or(source_time);
        let sequence = u32::try_from(attachment.sequence_number).unwrap_or(u32::MAX);
        let payload = sample.payload().to_bytes();

        let channel_id = match self.channel_mapping.get(&channel.topic).copied() {
            Some(channel_id) => channel_id,
            None => {
                let schema_id = self.writer.add_schema(
                    &channel.schema_name,
                    "ros-z-schema-json",
                    &channel.schema_data,
                )?;
                let channel_id = self.writer.add_channel(
                    schema_id,
                    &channel.topic,
                    "ros-z-cdr",
                    &channel.metadata,
                )?;
                self.channel_mapping
                    .insert(channel.topic.clone(), channel_id);
                channel_id
            }
        };

        self.writer.write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence,
                log_time: time_to_mcap_nanos(transport_time),
                publish_time: time_to_mcap_nanos(source_time),
            },
            payload.as_ref(),
        )?;

        self.last_written_source_time_by_topic
            .insert(channel.topic.clone(), source_time);

        Ok(true)
    }

    fn should_write_sample(&self, topic: &str, source_time: Time) -> bool {
        if topic != RAW_IMAGE_TOPIC {
            return true;
        }

        let Some(raw_image_min_interval) = self.raw_image_min_interval else {
            return true;
        };

        self.last_written_source_time_by_topic
            .get(topic)
            .is_none_or(|last_written_source_time| {
                source_time.duration_since(*last_written_source_time) >= raw_image_min_interval
            })
    }

    fn finish(mut self) -> Result<()> {
        self.writer.finish()?;
        Ok(())
    }
}

fn time_to_mcap_nanos(time: Time) -> u64 {
    u64::try_from(time.as_nanos()).unwrap_or_default()
}
