use crate::ReferenceFrame;
use ball_filter::tracker::UpdateSchedule;
use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{Isometry2, Point3, Pose2};
use projection::camera_matrix::CameraMatrix;
use ros_z::{SerdeCdrCodec, message::WireDecoder, time::Time};
use ros_z_streams::Announcement;
use serde::de::DeserializeOwned;
use std::{collections::BTreeMap, path::Path};
use types::{
    ball_position::BallPosition,
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

type Detections = Vec<Object<RobocupObjectLabel>>;

pub struct Input {
    pub time: Time,
    pub odometry: Option<Pose2<Odometry>>,
    pub detections: Option<Detections>,
    pub camera: Option<TimeWrapper<CameraMatrix>>,
}

#[derive(Clone)]
pub enum Reference {
    Ground(Vec<Point3<Ground>>),
    Field(Vec<Point3<Field>>),
}

impl Reference {
    pub fn validate_for_optimization(&self) -> Result<()> {
        let count = match self {
            Self::Ground(points) => points.len(),
            Self::Field(points) => points.len(),
        };
        ensure!(
            count <= 1,
            "ball-filter optimization requires at most one labelled ball per frame; multiple balls are supported only in the optional stress preview"
        );
        Ok(())
    }
}

pub struct Cycle {
    pub inputs: Vec<Input>,
    pub time: Time,
    pub dimensions: FieldDimensions,
    /// None means unlabelled, Some(empty) explicitly means no ball.
    pub reference: Option<Reference>,
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    /// Actual live prior decision, independently recorded from scoring geometry.
    /// Legacy recordings have no prior diagnostic and replay with no field prior.
    pub field_prior_pose: Option<Isometry2<Ground, Field>>,
    pub recorded_estimate: Option<BallPosition<Ground>>,
    pub seconds: f64,
}

pub struct Recording {
    pub cycles: Vec<Cycle>,
    pub path: String,
}

fn decode<T: DeserializeOwned>(message: &mcap::Message<'_>) -> Result<T> {
    ensure!(
        message.channel.message_encoding == "ros-z-cdr",
        "expected ros-z-cdr on {}",
        message.channel.topic
    );
    SerdeCdrCodec::<T>::deserialize(&message.data).wrap_err_with(|| {
        format!(
            "decoding {} at {}",
            message.channel.topic, message.publish_time
        )
    })
}

fn required<T: Clone>(map: &BTreeMap<Time, T>, time: Time, topic: &str) -> Result<T> {
    map.get(&time)
        .cloned()
        .ok_or_else(|| eyre!("missing {topic} at {time:?}; recording is incomplete"))
}

fn field_prior_pose_at(
    poses: &BTreeMap<Time, Option<Isometry2<Ground, Field>>>,
    time: Time,
) -> Result<Option<Isometry2<Ground, Field>>> {
    if poses.is_empty() {
        // Legacy captures predate the diagnostic. Never infer a live decision
        // from scoring geometry, which may have a different temporal selection.
        Ok(None)
    } else {
        // New captures publish even an explicit None for every output. A missing
        // entry is incomplete capture, not authorization to reuse a nearby pose.
        required(poses, time, "ball_filter/field_prior_pose")
    }
}

impl Recording {
    pub fn read(
        path: &Path,
        namespace: &str,
        reference_topic: &str,
        frame: ReferenceFrame,
    ) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let prefix = format!("{}/", namespace.trim_matches('/'));
        let mut odometry_payloads = BTreeMap::new();
        let mut odometry_announcements = Vec::new();
        let mut detection_announcements = Vec::new();
        let mut cameras = BTreeMap::new();
        let mut detection_payloads = BTreeMap::new();
        let mut references = BTreeMap::new();
        let mut ground_to_field = BTreeMap::new();
        let mut field_prior_poses = BTreeMap::new();
        let mut dimensions = BTreeMap::new();
        let mut estimates = BTreeMap::new();
        let mut schedules = BTreeMap::new();
        for message in mcap::MessageStream::new(&bytes)? {
            let message = message?;
            let topic = message.channel.topic.trim_start_matches('/');
            let topic = if namespace.is_empty() {
                topic
            } else {
                let Some(topic) = topic.strip_prefix(&prefix) else {
                    continue;
                };
                topic
            };
            let time = Time::from_nanos(i64::try_from(message.publish_time)?);
            match topic {
                "inputs/odometry" => {
                    ensure!(
                        odometry_payloads
                            .insert(message.sequence, decode::<Pose2<Odometry>>(&message)?)
                            .is_none(),
                        "multiple odometry publishers or duplicate sequences"
                    );
                }
                "inputs/odometry/announce" => {
                    odometry_announcements.push(decode::<Announcement>(&message)?);
                }
                "detected_objects/announce" => {
                    detection_announcements.push(decode::<Announcement>(&message)?);
                }
                "camera_matrix" => {
                    let camera: TimeWrapper<CameraMatrix> = decode(&message)?;
                    cameras.insert(camera.time, camera);
                }
                "detected_objects" => {
                    let objects: TimeWrapper<Detections> = decode(&message)?;
                    ensure!(
                        detection_payloads
                            .insert(message.sequence, objects.inner)
                            .is_none(),
                        "multiple detection publishers or duplicate sequences"
                    );
                }
                "ground_to_field" => {
                    ground_to_field.insert(time, decode::<Isometry2<Ground, Field>>(&message)?);
                }
                "ball_filter/field_prior_pose" => {
                    let pose: TimeWrapper<Option<Isometry2<Ground, Field>>> = decode(&message)?;
                    ensure!(
                        field_prior_poses.insert(pose.time, pose.inner).is_none(),
                        "duplicate ball_filter/field_prior_pose timestamp"
                    );
                }
                "field_dimensions" => {
                    dimensions.insert(time, decode::<FieldDimensions>(&message)?);
                }
                "ball_filter/ball_position" => {
                    estimates.insert(time, decode::<Option<BallPosition<Ground>>>(&message)?);
                }
                "ball_filter/update_schedule" => {
                    let schedule: UpdateSchedule = decode(&message)?;
                    ensure!(
                        schedules.insert(schedule.sequence, schedule).is_none(),
                        "duplicate filter sequence; use one episode per MCAP"
                    );
                }
                topic if topic == reference_topic => {
                    let (time, reference) = match frame {
                        ReferenceFrame::Ground => {
                            let value: TimeWrapper<Vec<Point3<Ground>>> = decode(&message)?;
                            (value.time, Reference::Ground(value.inner))
                        }
                        ReferenceFrame::Field => {
                            let value: TimeWrapper<Vec<Point3<Field>>> = decode(&message)?;
                            (value.time, Reference::Field(value.inner))
                        }
                    };
                    reference.validate_for_optimization()?;
                    references.insert(time, reference);
                }
                _ => {}
            }
        }
        ensure!(
            !schedules.is_empty(),
            "no ball_filter/update_schedule messages in {} (check --namespace)",
            path.display()
        );
        ensure!(
            !matches!(frame, ReferenceFrame::Field) || !ground_to_field.is_empty(),
            "field scoring requires recorded ground_to_field messages"
        );
        let odometry =
            pair_announcements(odometry_payloads, odometry_announcements, "inputs/odometry")?;
        let detections = pair_announcements(
            detection_payloads,
            detection_announcements,
            "detected_objects",
        )?;
        let mut cycles = Vec::new();
        let mut previous_time = None;
        for (index, (sequence, schedule)) in schedules.into_iter().enumerate() {
            ensure!(
                sequence == index as u64,
                "missing filter update {index}; start recording before advancing the robot clock, with a fresh filter"
            );
            let time = schedule
                .inputs
                .last()
                .ok_or_else(|| eyre!("empty filter update"))?
                .time;
            ensure!(
                previous_time.is_none_or(|previous| time > previous),
                "non-monotonic filter update"
            );
            let seconds =
                previous_time.map_or(0.0, |previous| time.duration_since(previous).as_secs_f64());
            previous_time = Some(time);
            let mut inputs = Vec::new();
            for stamp in schedule.inputs {
                inputs.push(Input {
                    time: stamp.time,
                    odometry: stamp
                        .odometry
                        .then(|| required(&odometry, stamp.time, "inputs/odometry"))
                        .transpose()?,
                    detections: stamp
                        .detections
                        .then(|| required(&detections, stamp.time, "detected_objects"))
                        .transpose()?,
                    camera: if stamp.detections {
                        stamp
                            .camera_time
                            .map(|t| required(&cameras, t, "camera_matrix"))
                            .transpose()?
                    } else {
                        None
                    },
                });
            }
            cycles.push(Cycle {
                inputs,
                time,
                seconds,
                dimensions: dimensions
                    .range(..=time)
                    .next_back()
                    .ok_or_else(|| eyre!("missing field_dimensions"))?
                    .1
                    .clone(),
                reference: references.get(&time).cloned(),
                // Use a preceding pose with a bounded source-time age. Never apply
                // an arbitrarily old transform to a fresh ball estimate.
                ground_to_field: ground_to_field
                    .range(..=time)
                    .next_back()
                    .filter(|(stamp, _)| {
                        time.duration_since(**stamp) <= std::time::Duration::from_millis(20)
                    })
                    .map(|(_, transform)| *transform),
                recorded_estimate: required(&estimates, time, "ball_filter/ball_position")?,
                field_prior_pose: field_prior_pose_at(&field_prior_poses, time)?,
            });
        }
        ensure!(
            cycles
                .iter()
                .any(|c| c.reference.is_some() && c.seconds > 0.0),
            "no timestamp-matched ground truth; unlabelled frames cannot be scored"
        );
        Ok(Self {
            cycles,
            path: path.display().to_string(),
        })
    }
}

// MCAP stores the payload publication sequence. The announcement carries the
// sensor/fusion timestamp; using MCAP publish_time here changes odometry timing.
fn pair_announcements<T>(
    mut payloads: BTreeMap<u32, T>,
    announcements: Vec<Announcement>,
    topic: &str,
) -> Result<BTreeMap<Time, T>> {
    ensure!(
        !announcements.is_empty(),
        "missing {topic}/announce messages"
    );
    let mut publisher = None;
    let mut result = BTreeMap::new();
    for announcement in announcements {
        ensure!(
            publisher.is_none_or(|p| p == announcement.source_global_id()),
            "multiple publishers on {topic} are not supported in one episode"
        );
        publisher = Some(announcement.source_global_id());
        let sequence = u32::try_from(announcement.sequence_number())?;
        ensure!(
            sequence < u32::MAX,
            "ambiguous clamped MCAP sequence on {topic}"
        );
        // An announced payload may not have arrived by capture end. If the filter
        // actually consumed it, the schedule's required() lookup rejects the gap.
        if let Some(payload) = payloads.remove(&sequence) {
            ensure!(
                result.insert(announcement.time(), payload).is_none(),
                "duplicate fusion timestamp on {topic}"
            );
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::message::WireEncoder;

    #[test]
    fn field_prior_diagnostic_replays_exact_decisions_and_preserves_legacy_none() {
        type Diagnostic = TimeWrapper<Option<Isometry2<Ground, Field>>>;
        let time = Time::from_nanos(100);
        let pose = Isometry2::from_parts(linear_algebra::vector![2.0, -1.0], 0.7);
        let mut poses = BTreeMap::new();
        assert!(field_prior_pose_at(&poses, time).unwrap().is_none());
        for (stamp, selected) in [(time, Some(pose)), (Time::from_nanos(200), None)] {
            let bytes = SerdeCdrCodec::<Diagnostic>::serialize(&TimeWrapper {
                time: stamp,
                inner: selected,
            })
            .unwrap();
            let decoded = SerdeCdrCodec::<Diagnostic>::deserialize(&bytes).unwrap();
            assert_eq!(decoded.time, stamp);
            poses.insert(decoded.time, decoded.inner);
        }
        let replay = field_prior_pose_at(&poses, time).unwrap().unwrap();
        let point = linear_algebra::point![0.4, -0.2];
        assert!(((replay * point) - (pose * point)).norm() < 1e-6);
        assert!(
            field_prior_pose_at(&poses, Time::from_nanos(200))
                .unwrap()
                .is_none()
        );
        // Even a neighbouring diagnostic is not the actual decision for this
        // output. Partial captures must fail instead of using a nearest pose.
        assert!(field_prior_pose_at(&poses, Time::from_nanos(201)).is_err());
    }

    #[test]
    fn optimization_rejects_ambiguous_multiball_labels_in_both_frames() {
        for count in [0, 1, 2, 3] {
            let ground = Reference::Ground(vec![linear_algebra::point![0.0, 0.0, 0.1]; count]);
            let field = Reference::Field(vec![linear_algebra::point![0.0, 0.0, 0.1]; count]);
            for reference in [ground, field] {
                let result = reference.validate_for_optimization();
                assert_eq!(result.is_ok(), count <= 1);
                if let Err(error) = result {
                    assert!(error.to_string().contains("optional stress preview"));
                }
            }
        }
    }

    fn announcement(time: i64, sequence: i64, publisher: u8) -> Announcement {
        serde_json::from_value(serde_json::json!({
            "time": Time::from_nanos(time),
            "sequence_number": sequence,
            "source_global_id": ros_z::EndpointGlobalId::from([publisher; 16]),
        }))
        .unwrap()
    }

    #[test]
    fn pairs_by_publication_sequence_and_uses_announced_sensor_time() {
        let payloads = BTreeMap::from([(7, "first"), (8, "second")]);
        let inputs = pair_announcements(
            payloads,
            vec![announcement(200, 8, 1), announcement(100, 7, 1)],
            "test",
        )
        .unwrap();
        assert_eq!(
            inputs.into_iter().collect::<Vec<_>>(),
            vec![
                (Time::from_nanos(100), "first"),
                (Time::from_nanos(200), "second"),
            ]
        );
    }

    #[test]
    fn refuses_ambiguous_publisher_or_timestamp() {
        for announcements in [
            vec![announcement(100, 7, 1), announcement(200, 8, 2)],
            vec![announcement(100, 7, 1), announcement(100, 8, 1)],
        ] {
            assert!(
                pair_announcements(BTreeMap::from([(7, 1), (8, 2)]), announcements, "test")
                    .is_err()
            );
        }
    }
}
