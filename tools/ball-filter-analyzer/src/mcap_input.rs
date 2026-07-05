use std::{collections::BTreeMap, fs, path::Path};

use color_eyre::{Result, eyre::WrapErr};
use coordinate_systems::Ground;
use mcap::MessageStream;
use projection::camera_matrix::CameraMatrix;
use ros_z::{SerdeCdrCodec, message::WireDecoder};
use serde::de::DeserializeOwned;
use types::{
    ball_detection::BallPercept,
    ball_position::BallPosition,
    ball_tracking::{BallTrack, TrackerDebugState},
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use crate::{
    records::{
        DebugRow, PerceptRow, PrimaryRow, TopicHealthRow, TrackRow, debug_row, percept_rows,
        primary_row, track_rows,
    },
    topic_health::TopicStats,
};

pub const RELEVANT_TOPICS: &[(&str, bool)] = &[
    ("inputs/odometry", true),
    ("camera_matrix", true),
    ("detected_objects", true),
    ("detected_objects/announce", false),
    ("ball_filter/ball_percepts", false),
    ("ball_filter/tracks", false),
    ("ball_filter/primary_ball", false),
    ("ball_filter/debug_state", false),
    ("ball_filter/filtered_balls_in_image", false),
    ("ball_filter/ball_position", false),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelevantTopic {
    Odometry,
    CameraMatrix,
    DetectedObjects,
    DetectedObjectsAnnounce,
    BallPercepts,
    Tracks,
    PrimaryBall,
    DebugState,
    FilteredBallsInImage,
    BallPosition,
}

#[derive(Clone, Debug, Default)]
pub struct DecodeErrorRow {
    pub topic: String,
    pub time_ns: i64,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct AnalysisInput {
    pub topic_health: Vec<TopicHealthRow>,
    pub percepts: Vec<PerceptRow>,
    pub tracks: Vec<TrackRow>,
    pub primary: Vec<PrimaryRow>,
    pub debug: Vec<DebugRow>,
    pub decode_errors: Vec<DecodeErrorRow>,
}

pub fn relevant_topic(topic: &str) -> Option<RelevantTopic> {
    match topic {
        "inputs/odometry" => Some(RelevantTopic::Odometry),
        "camera_matrix" => Some(RelevantTopic::CameraMatrix),
        "detected_objects" => Some(RelevantTopic::DetectedObjects),
        "detected_objects/announce" => Some(RelevantTopic::DetectedObjectsAnnounce),
        "ball_filter/ball_percepts" => Some(RelevantTopic::BallPercepts),
        "ball_filter/tracks" => Some(RelevantTopic::Tracks),
        "ball_filter/primary_ball" => Some(RelevantTopic::PrimaryBall),
        "ball_filter/debug_state" => Some(RelevantTopic::DebugState),
        "ball_filter/filtered_balls_in_image" => Some(RelevantTopic::FilteredBallsInImage),
        "ball_filter/ball_position" => Some(RelevantTopic::BallPosition),
        _ => None,
    }
}

pub fn read_recording(path: &Path) -> Result<AnalysisInput> {
    let bytes = fs::read(path)
        .wrap_err_with(|| format!("failed to read MCAP recording {}", path.display()))?;
    let mut stats_by_topic = BTreeMap::<String, TopicStats>::new();
    for (topic, _) in RELEVANT_TOPICS {
        stats_by_topic.insert((*topic).to_string(), TopicStats::default());
    }

    let mut output = AnalysisInput::default();
    let mut decoded_relevant_messages = 0_usize;
    let stream = MessageStream::new(&bytes).wrap_err("failed to open MCAP message stream")?;
    for message in stream {
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                if decoded_relevant_messages > 0 || has_useful_decoded_data(&output) {
                    output
                        .decode_errors
                        .push(mcap_stream_error(error.to_string()));
                    break;
                }
                return Err(error).wrap_err("failed to read MCAP message");
            }
        };
        let topic = message.channel.topic.as_str();
        let Some(relevant_topic) = relevant_topic(topic) else {
            continue;
        };
        let time_ns = i64::try_from(message.publish_time).unwrap_or(i64::MAX);
        stats_by_topic
            .entry(topic.to_string())
            .or_default()
            .observe(time_ns);

        match relevant_topic {
            RelevantTopic::BallPercepts => match decode::<Vec<BallPercept>>(&message) {
                Ok(percepts) => {
                    decoded_relevant_messages += 1;
                    output.percepts.extend(percept_rows(time_ns, &percepts));
                }
                Err(error) => output
                    .decode_errors
                    .push(decode_error(topic, time_ns, error)),
            },
            RelevantTopic::Tracks => match decode::<Vec<BallTrack<Ground>>>(&message) {
                Ok(tracks) => {
                    decoded_relevant_messages += 1;
                    output.tracks.extend(track_rows(time_ns, &tracks));
                }
                Err(error) => output
                    .decode_errors
                    .push(decode_error(topic, time_ns, error)),
            },
            RelevantTopic::PrimaryBall => match decode::<Option<BallTrack<Ground>>>(&message) {
                Ok(primary) => {
                    decoded_relevant_messages += 1;
                    output.primary.push(primary_row(time_ns, &primary));
                }
                Err(error) => output
                    .decode_errors
                    .push(decode_error(topic, time_ns, error)),
            },
            RelevantTopic::DebugState => match decode::<TrackerDebugState>(&message) {
                Ok(debug) => {
                    decoded_relevant_messages += 1;
                    output.debug.push(debug_row(time_ns, &debug));
                }
                Err(error) => output
                    .decode_errors
                    .push(decode_error(topic, time_ns, error)),
            },
            RelevantTopic::CameraMatrix => record_decode_health::<TimeWrapper<CameraMatrix>>(
                &message,
                topic,
                time_ns,
                &mut output.decode_errors,
                &mut decoded_relevant_messages,
            ),
            RelevantTopic::DetectedObjects => {
                record_decode_health::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(
                    &message,
                    topic,
                    time_ns,
                    &mut output.decode_errors,
                    &mut decoded_relevant_messages,
                )
            }
            RelevantTopic::BallPosition => record_decode_health::<Option<BallPosition<Ground>>>(
                &message,
                topic,
                time_ns,
                &mut output.decode_errors,
                &mut decoded_relevant_messages,
            ),
            RelevantTopic::Odometry
            | RelevantTopic::DetectedObjectsAnnounce
            | RelevantTopic::FilteredBallsInImage => {}
        }
    }

    output.topic_health = RELEVANT_TOPICS
        .iter()
        .map(|(topic, required)| {
            stats_by_topic
                .get(*topic)
                .cloned()
                .unwrap_or_default()
                .to_row(topic, *required)
        })
        .collect();

    Ok(output)
}

fn has_useful_decoded_data(output: &AnalysisInput) -> bool {
    !output.percepts.is_empty()
        || !output.tracks.is_empty()
        || !output.primary.is_empty()
        || !output.debug.is_empty()
}

fn decode<T>(message: &mcap::Message<'_>) -> Result<T, ros_z::message::CdrError>
where
    T: DeserializeOwned,
{
    SerdeCdrCodec::<T>::deserialize(message.data.as_ref())
}

fn record_decode_health<T>(
    message: &mcap::Message<'_>,
    topic: &str,
    time_ns: i64,
    decode_errors: &mut Vec<DecodeErrorRow>,
    decoded_relevant_messages: &mut usize,
) where
    T: DeserializeOwned,
{
    match decode::<T>(message) {
        Ok(_) => *decoded_relevant_messages += 1,
        Err(error) => decode_errors.push(decode_error(topic, time_ns, error)),
    }
}

fn decode_error(topic: &str, time_ns: i64, error: ros_z::message::CdrError) -> DecodeErrorRow {
    DecodeErrorRow {
        topic: topic.to_string(),
        time_ns,
        message: error.to_string(),
    }
}

fn mcap_stream_error(message: String) -> DecodeErrorRow {
    DecodeErrorRow {
        topic: "<mcap>".to_string(),
        time_ns: -1,
        message,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::records::TrackRow;

    use super::{
        AnalysisInput, RelevantTopic, has_useful_decoded_data, read_recording, relevant_topic,
    };

    #[test]
    fn relevant_topic_classifies_ball_filter_tracks() {
        assert_eq!(
            relevant_topic("ball_filter/tracks"),
            Some(RelevantTopic::Tracks)
        );
    }

    #[test]
    fn relevant_topic_ignores_unrelated_topics() {
        assert_eq!(relevant_topic("behavior/motion_command"), None);
    }

    #[test]
    fn read_recording_errors_on_missing_input() {
        let path = temp_recording_path();

        let result = read_recording(&path);

        assert!(result.is_err());
    }

    #[test]
    fn read_recording_errors_on_invalid_mcap_without_useful_data() {
        let path = temp_recording_path();
        fs::write(&path, b"\x89MCAP0\r\n\x01").unwrap();

        let result = read_recording(&path);

        fs::remove_file(path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn has_useful_decoded_data_returns_true_when_relevant_rows_were_decoded() {
        let input = AnalysisInput {
            tracks: vec![TrackRow {
                time_ns: 1,
                track_id: 1,
                status: String::new(),
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                existence: 1.0,
                cov_xx: 0.0,
                cov_xy: 0.0,
                cov_yy: 0.0,
                last_seen_ns: 1,
            }],
            ..Default::default()
        };

        assert!(has_useful_decoded_data(&input));
    }

    fn temp_recording_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "ball-filter-analyzer-test-{}.mcap",
            std::process::id()
        ))
    }
}
