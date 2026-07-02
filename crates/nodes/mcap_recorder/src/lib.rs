use std::{
    boxed::Box,
    collections::BTreeMap,
    fs::{self, File},
    future::Future,
    io::BufWriter,
    num::NonZeroUsize,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use booster::{ImuState, MotorState};
use color_eyre::{Result, eyre::WrapErr};
use coordinate_systems::{Field, Ground, Robot};
use field_mark_association::{FieldMarkAssociations, GlobalLocalizationDebug};
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use kinematics::joints::{Joints, head::HeadJoints};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use localization_3d::SolveDiagnostics;
use mcap::{Compression, WriteOptions, Writer, records::MessageHeader};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{
    Message,
    attachment::Attachment,
    prelude::*,
    pubsub::RawSubscriber,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use ros_z_streams::Announcement;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::MotionCommand,
    object_detection::{Object, RobocupObjectLabel},
    stereo_image_pair::StereoImagePair,
    support_foot::Side,
    time_wrapper::TimeWrapper,
    visual_odometry::{VisualOdometer, VisualOdometryDelta},
};
use zenoh::sample::Sample;

type ChannelId = u16;
type RecorderTasks = FuturesUnordered<BoxFuture<'static, Result<RecordedSample>>>;
const RAW_IMAGE_TOPIC: &str = "inputs/stereo_image_pair";

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct McapRecorderParameters {
    pub enable: bool,
    pub output_path: PathBuf,
    pub max_duration: Option<Duration>,
    pub include_raw_images: bool,
    pub raw_image_min_interval: Option<Duration>,
    pub queue_depth: usize,
}

impl McapRecorderParameters {
    fn validate(&self) -> std::result::Result<(), String> {
        if self.output_path.as_os_str().is_empty() {
            return Err("output_path must not be empty".to_string());
        }
        if self.max_duration.is_some_and(|duration| duration.is_zero()) {
            return Err("max_duration must be positive when set".to_string());
        }
        if self
            .raw_image_min_interval
            .is_some_and(|duration| duration.is_zero())
        {
            return Err("raw_image_min_interval must be positive when set".to_string());
        }
        if self.queue_depth == 0 {
            return Err("queue_depth must be positive".to_string());
        }
        Ok(())
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("mcap_recorder").build().await?;
    let parameters = node.bind_parameter_as::<McapRecorderParameters>("mcap_recorder")?;
    parameters.add_validation_hook(McapRecorderParameters::validate)?;
    let parameters = parameters.snapshot().typed().clone();

    if !parameters.enable {
        std::future::pending::<()>().await;
        return Ok(());
    }

    if let Some(parent) = parameters.output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).wrap_err_with(|| {
            format!(
                "failed to create recorder output directory {}",
                parent.display()
            )
        })?;
    }

    let file = File::create(&parameters.output_path).wrap_err_with(|| {
        format!(
            "failed to create localization recording {}",
            parameters.output_path.display()
        )
    })?;
    let writer = McapWriter::new(BufWriter::new(file), parameters.raw_image_min_interval)?;
    let mut writer = writer;

    let mut recorders = RecorderTasks::new();
    subscribe_topic::<ImuState>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "inputs/imu_state",
    )
    .await?;
    subscribe_topic::<VisualOdometryDelta>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "visual_odometry/current_left_camera_to_previous_left_camera",
    )
    .await?;
    subscribe_topic::<TimeWrapper<RobotKinematics>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "robot_kinematics",
    )
    .await?;
    subscribe_topic::<TimeWrapper<Option<Side>>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "support_foot",
    )
    .await?;
    subscribe_topic::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "ground_to_robot",
    )
    .await?;
    subscribe_topic::<TimeWrapper<CameraMatrix>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "camera_matrix",
    )
    .await?;
    subscribe_topic::<Vec<Object<RobocupObjectLabel>>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "detected_objects",
    )
    .await?;
    subscribe_topic::<Announcement>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "detected_objects/announce",
    )
    .await?;
    subscribe_topic::<FieldDimensions>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "field_dimensions",
    )
    .await?;
    subscribe_topic::<Option<Isometry3<Field, Robot>>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "localization",
    )
    .await?;
    subscribe_topic::<VisualOdometer>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "visual_odometry/current_left_camera_to_visual_odometer",
    )
    .await?;
    subscribe_topic::<Option<nalgebra::Isometry3<f32>>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "debug/visual_odometry/previous_left_camera_to_current_left_camera",
    )
    .await?;
    subscribe_topic::<Option<GlobalLocalizationDebug>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "debug/global_localization",
    )
    .await?;
    subscribe_topic::<TimeWrapper<FieldMarkAssociations>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "field_mark_association/associations",
    )
    .await?;
    subscribe_topic::<Intrinsic>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "debug/calibrated_intrinsics",
    )
    .await?;
    subscribe_topic::<TimeWrapper<SolveDiagnostics>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "debug/solve_diagnostics",
    )
    .await?;
    subscribe_topic::<Joints<MotorState>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "inputs/serial_motor_states",
    )
    .await?;
    subscribe_topic::<MotionCommand>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "behavior/motion_command",
    )
    .await?;
    subscribe_topic::<MotionCommand>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "motion_command",
    )
    .await?;
    subscribe_topic::<HeadJoints<f32>>(&node, &mut recorders, parameters.queue_depth, "look_at")
        .await?;
    subscribe_topic::<HeadJoints<f32>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "look_around_target_joints",
    )
    .await?;
    subscribe_topic::<HeadJoints<f32>>(
        &node,
        &mut recorders,
        parameters.queue_depth,
        "head_joints_command",
    )
    .await?;

    if parameters.include_raw_images {
        subscribe_topic::<TimeWrapper<StereoImagePair>>(
            &node,
            &mut recorders,
            parameters.queue_depth,
            RAW_IMAGE_TOPIC,
        )
        .await?;
    }

    tracing::info!(
        path = %parameters.output_path.display(),
        include_raw_images = parameters.include_raw_images,
        raw_image_min_interval = ?parameters.raw_image_min_interval,
        queue_depth = parameters.queue_depth,
        compression = "lz4",
        "localization recording started"
    );

    let mut samples_written = 0;
    let recorder_result = record_samples(
        &mut recorders,
        &mut writer,
        parameters.max_duration,
        &mut samples_written,
    )
    .await;
    let finish_result = writer.finish();
    tracing::info!(
        path = %parameters.output_path.display(),
        samples_written,
        "localization recording finished"
    );

    recorder_result?;
    finish_result?;

    Ok(())
}

async fn subscribe_topic<T>(
    node: &Node,
    recorders: &mut RecorderTasks,
    queue_depth: usize,
    topic: &'static str,
) -> Result<()>
where
    T: Message + Send + Sync + 'static,
{
    let subscriber = node
        .subscriber::<T>(topic)
        .raw()
        .qos(recorder_qos(queue_depth))
        .build()
        .await?;
    let recorder = TopicRecorder {
        subscriber,
        channel: RecordedChannel::for_message::<T>(topic),
    };

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
    writer: &mut McapWriter<BufWriter<File>>,
    max_duration: Option<Duration>,
    samples_written: &mut usize,
) -> Result<()> {
    if let Some(max_duration) = max_duration {
        let timer = tokio::time::sleep(max_duration);
        tokio::pin!(timer);

        loop {
            tokio::select! {
                _ = &mut timer => return Ok(()),
                result = recorders.next() => {
                    if !handle_recorded_sample(result, recorders, writer, samples_written)? {
                        return Ok(());
                    }
                }
            }
        }
    }

    while let Some(result) = recorders.next().await {
        handle_recorded_sample(Some(result), recorders, writer, samples_written)?;
    }

    Ok(())
}

fn handle_recorded_sample(
    result: Option<Result<RecordedSample>>,
    recorders: &mut RecorderTasks,
    writer: &mut McapWriter<BufWriter<File>>,
    samples_written: &mut usize,
) -> Result<bool> {
    let Some(result) = result else {
        return Ok(false);
    };
    let RecordedSample { recorder, sample } = result?;

    if writer.write(&recorder.channel, &sample)? {
        *samples_written += 1;
    }
    recorders.push(receive_sample(recorder));

    Ok(true)
}

struct RecordedChannel {
    topic: &'static str,
    schema_name: String,
    schema_data: Vec<u8>,
    metadata: BTreeMap<String, String>,
}

impl RecordedChannel {
    fn for_message<T: Message>(topic: &'static str) -> Self {
        let schema_name = T::type_name();
        let schema_data = serde_json::to_vec(&T::schema()).unwrap_or_default();
        let mut metadata = BTreeMap::new();
        metadata.insert("ros_z.type_name".to_string(), schema_name.clone());
        metadata.insert(
            "ros_z.schema_hash".to_string(),
            T::schema_hash().to_hash_string(),
        );

        Self {
            topic,
            schema_name,
            schema_data,
            metadata,
        }
    }
}

struct TopicRecorder {
    subscriber: RawSubscriber,
    channel: RecordedChannel,
}

struct RecordedSample {
    recorder: TopicRecorder,
    sample: Sample,
}

struct McapWriter<W: std::io::Write + std::io::Seek> {
    writer: Writer<W>,
    channel_mapping: BTreeMap<&'static str, ChannelId>,
    last_written_source_time_by_topic: BTreeMap<&'static str, Time>,
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
        if !self.should_write_sample(channel.topic, source_time) {
            return Ok(false);
        }
        let transport_time = sample
            .timestamp()
            .map(|timestamp| Time::from_wallclock(timestamp.get_time().to_system_time()))
            .unwrap_or(source_time);
        let sequence = u32::try_from(attachment.sequence_number).unwrap_or(u32::MAX);
        let payload = sample.payload().to_bytes();

        let channel_id = match self.channel_mapping.get(channel.topic).copied() {
            Some(channel_id) => channel_id,
            None => {
                let schema_id = self.writer.add_schema(
                    &channel.schema_name,
                    "ros-z-schema-json",
                    &channel.schema_data,
                )?;
                let channel_id = self.writer.add_channel(
                    schema_id,
                    channel.topic,
                    "ros-z-cdr",
                    &channel.metadata,
                )?;
                self.channel_mapping.insert(channel.topic, channel_id);
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
            .insert(channel.topic, source_time);

        Ok(true)
    }

    fn should_write_sample(&self, topic: &'static str, source_time: Time) -> bool {
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
