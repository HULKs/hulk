//! Replay current association/localization with recorded solve times and resets.
//! Usage: relocalization_replay recording.mcap
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use booster::{ImuState, LowState};
use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use field_mark_association::{
    AssociationGeometry, AssociationInput, FieldMarkAssociationParameters,
    associate_visual_features, find_detected_visual_features,
};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Orientation3;
use localization_3d::{Localization, Localization3dParameters, initial_robot_to_local_from_imu};
use mcap::{
    records::Record,
    sans_io::{LinearReadEvent, LinearReader},
};
use ros_z::{CdrCodec, message::WireDecoder, time::Time};
use types::{
    camera_geometry::{CameraGeometry, interpolate_camera_geometry},
    field_dimensions::FieldDimensions,
    localization::{LocalizationEstimate, LocalizationStatus},
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
    visual_localization::VisualLocalizationFrame,
    visual_odometry::VisualOdometer,
};

const REQUIRED_TOPICS: &[&str] = &[
    "field_dimensions",
    "inputs/low_state",
    "robot_kinematics",
    "camera_geometry",
    "visual_odometry/current_left_camera_to_visual_odometer",
    "detected_objects",
    "localization/status",
    "debug/solve_diagnostics",
];

fn require_topics(seen: &BTreeSet<String>) -> Result<()> {
    let missing: Vec<_> = REQUIRED_TOPICS
        .iter()
        .filter(|topic| !seen.contains(**topic))
        .collect();
    ensure!(
        missing.is_empty(),
        "missing required topic messages: {missing:?}"
    );
    Ok(())
}

enum Event {
    Imu(Time, ImuState),
    Feet(Box<TimeWrapper<RobotKinematics>>),
    Camera(TimeWrapper<CameraGeometry>),
    Vo(VisualOdometer),
    Objects(TimeWrapper<Vec<Object<RobocupObjectLabel>>>),
    Reset(LocalizationStatus),
    Solve,
}
enum Measurement {
    Imu(Time, ImuState),
    Feet(Box<TimeWrapper<RobotKinematics>>),
    Vo(VisualOdometer),
    Visual(TimeWrapper<VisualLocalizationFrame>),
}
impl Measurement {
    fn time(&self) -> Time {
        match self {
            Self::Imu(t, _) => *t,
            Self::Feet(v) => v.time,
            Self::Vo(v) => v.time,
            Self::Visual(v) => v.time,
        }
    }
}
fn camera_at(
    cache: &BTreeMap<Time, TimeWrapper<CameraGeometry>>,
    t: Time,
) -> Result<CameraGeometry> {
    interpolate_camera_geometry(
        cache
            .range(..=t)
            .next_back()
            .ok_or_else(|| eyre!("missing camera geometry before {t:?}"))?
            .1,
        cache
            .range(t..)
            .next()
            .ok_or_else(|| eyre!("missing camera geometry after {t:?}"))?
            .1,
        t,
        Duration::from_millis(20),
    )
    .ok_or_else(|| eyre!("invalid camera geometry bracket at {t:?}"))
}
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .init();
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or_else(|| eyre!("usage: relocalization_replay recording.mcap"))?;
    ensure!(
        args.next().is_none(),
        "usage: relocalization_replay recording.mcap"
    );
    let mut file = File::open(path)?;
    let mut reader = LinearReader::new();
    let mut topics = BTreeMap::new();
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    let mut field = None;
    while let Some(event) = reader.next_event() {
        match event? {
            LinearReadEvent::ReadRequest(n) => {
                let n = file.read(reader.insert(n))?;
                ensure!(n > 0, "MCAP ended before the reader completed");
                reader.notify_read(n);
            }
            LinearReadEvent::Record { opcode, data } => match mcap::parse_record(opcode, data)? {
                Record::Channel(c) => {
                    topics.insert(c.id, c.topic);
                }
                Record::Message { header, data } => {
                    let topic = topics
                        .get(&header.channel_id)
                        .ok_or_else(|| eyre!("unknown MCAP channel {}", header.channel_id))?;
                    let event = match topic.as_str() {
                        "field_dimensions" => {
                            let dimensions = CdrCodec::<FieldDimensions>::deserialize(&data)?;
                            ensure!(
                                field
                                    .as_ref()
                                    .is_none_or(|previous| previous == &dimensions),
                                "field dimensions changed during recording"
                            );
                            field = Some(dimensions);
                            None
                        }
                        "inputs/low_state" => Some(Event::Imu(
                            Time::from_nanos(header.publish_time as i64),
                            CdrCodec::<LowState>::deserialize(&data)?.imu_state,
                        )),
                        "robot_kinematics" => Some(Event::Feet(Box::new(CdrCodec::<
                            TimeWrapper<RobotKinematics>,
                        >::deserialize(
                            &data
                        )?))),
                        "camera_geometry" => {
                            Some(Event::Camera(
                                CdrCodec::<TimeWrapper<CameraGeometry>>::deserialize(&data)?,
                            ))
                        }
                        "visual_odometry/current_left_camera_to_visual_odometer" => Some(
                            Event::Vo(CdrCodec::<VisualOdometer>::deserialize(&data)?),
                        ),
                        "detected_objects" => Some(Event::Objects(CdrCodec::<
                            TimeWrapper<Vec<Object<RobocupObjectLabel>>>,
                        >::deserialize(
                            &data
                        )?)),
                        "localization/status" => {
                            Some(Event::Reset(
                                CdrCodec::<LocalizationStatus>::deserialize(&data)?,
                            ))
                        }
                        "debug/solve_diagnostics" => Some(Event::Solve),
                        _ => continue,
                    };
                    seen.insert(topic.clone());
                    if let Some(event) = event {
                        events.push((header.log_time, event));
                    }
                }
                _ => (),
            },
        }
    }
    require_topics(&seen)?;
    let field = field.ok_or_else(|| eyre!("missing field_dimensions"))?;
    events.sort_by_key(|e| e.0);
    let parameters: Localization3dParameters = json5::from_str(include_str!(
        "../../../../etc/parameters/base/localization3d.json5"
    ))?;
    let association_parameters: FieldMarkAssociationParameters = json5::from_str(include_str!(
        "../../../../etc/parameters/base/field_mark_association.json5"
    ))?;
    let mut active: Option<Localization> = None;
    let mut reset: Option<LocalizationStatus> = None;
    let mut cameras = BTreeMap::new();
    let mut attitudes = BTreeMap::new();
    let mut feet: Option<TimeWrapper<RobotKinematics>> = None;
    let mut pending = Vec::new();
    let mut estimates: BTreeMap<Time, LocalizationEstimate> = BTreeMap::new();
    let mut output = Vec::new();
    let mut audit_frames = Vec::new();
    let mut solve_us = Vec::new();
    let mut association_us = Vec::new();
    let mut previous_status = None;
    let mut lifecycle = Vec::new();
    let started = Instant::now();
    for (delivery, event) in events {
        let now = Time::from_nanos(delivery as i64);
        match event {
            Event::Reset(s) => {
                if reset
                    .as_ref()
                    .is_none_or(|previous| previous.epoch != s.epoch)
                {
                    reset = Some(s);
                    active = None;
                    pending.clear();
                    estimates.clear();
                }
            }
            Event::Camera(c) => {
                cameras.insert(c.time, c);
                cameras.retain(|t, _| t.saturating_add(Duration::from_secs(4)) >= now);
            }
            Event::Feet(f) => {
                feet = Some((*f).clone());
                pending.push(Measurement::Feet(f));
            }
            Event::Imu(t, imu) => {
                let rpy = imu.roll_pitch_yaw.inner;
                attitudes.insert(t, Orientation3::from_euler_angles(rpy.x, rpy.y, rpy.z));
                attitudes.retain(|t, _| t.saturating_add(Duration::from_secs(4)) >= now);
                pending.push(Measurement::Imu(t, imu));
            }
            Event::Vo(v) => pending.push(Measurement::Vo(v)),
            Event::Objects(objects) => {
                // No field association is possible before initialization.
                let Some(localization) = active.as_mut() else {
                    continue;
                };
                localization.advance_time(now);
                let status = localization.status();
                let epoch = reset
                    .as_ref()
                    .ok_or_else(|| eyre!("detections before initial status"))?;
                let t = objects.time;
                if t < epoch.time || t.saturating_add(Duration::from_millis(250)) < now {
                    continue;
                }
                let camera = camera_at(&cameras, t)?;
                let attitude = attitudes
                    .range(..=t)
                    .next_back()
                    .zip(attitudes.range(t..).next())
                    .and_then(|((a, ra), (b, rb))| {
                        let gap = b.duration_since(*a);
                        (gap <= Duration::from_millis(20)).then(|| {
                            if gap.is_zero() {
                                *ra
                            } else {
                                ra.slerp(
                                    *rb,
                                    t.duration_since(*a).as_secs_f32() / gap.as_secs_f32(),
                                )
                            }
                        })
                    })
                    .ok_or_else(|| eyre!("missing IMU attitude bracket within 20 ms at {t:?}"))?;
                let geometry = estimates
                    .range(..=t)
                    .next_back()
                    .filter(|(stamp, _)| stamp.saturating_add(Duration::from_millis(250)) >= t)
                    .and_then(|(_, e)| AssociationGeometry::from_estimate(e, &status));
                let features = find_detected_visual_features(&objects.inner);
                let timer = Instant::now();
                let result = associate_visual_features(
                    AssociationInput {
                        status: &status,
                        attitude: Some(attitude),
                        tracking: geometry.as_ref(),
                        visual_features: &features,
                        robot_to_camera: camera.robot_to_camera,
                        camera_intrinsic: camera.intrinsics,
                        field_dimensions: &field,
                        time: t,
                    },
                    &association_parameters,
                );
                association_us.push(timer.elapsed().as_secs_f64() * 1e6);
                if !result.associations.is_empty() {
                    pending.push(Measurement::Visual(TimeWrapper {
                        time: t,
                        inner: VisualLocalizationFrame {
                            epoch: epoch.epoch,
                            generation: status.generation,
                            source: result.source,
                            robot_to_camera: camera.robot_to_camera,
                            camera_intrinsic: camera.intrinsics,
                            associations: result.associations,
                        },
                    }));
                }
            }
            Event::Solve => {
                let epoch = reset
                    .as_ref()
                    .ok_or_else(|| eyre!("solve before initial localization/status"))?;
                let timer = Instant::now();
                pending.sort_by_key(Measurement::time);
                for m in pending.drain(..) {
                    let t = m.time();
                    if t < epoch.time || t > now {
                        continue;
                    }
                    if active.is_none() {
                        let Measurement::Imu(_, ref imu) = m else {
                            continue;
                        };
                        let (_, camera) = cameras
                            .range(..=t)
                            .next_back()
                            .filter(|(stamp, _)| stamp.abs_diff(t) <= Duration::from_millis(20))
                            .ok_or_else(|| {
                                eyre!("missing initialization camera geometry at {t:?}")
                            })?;
                        let feet = feet.as_ref().ok_or_else(|| {
                            eyre!("missing initialization robot kinematics at {t:?}")
                        })?;
                        active = Some(Localization::new(
                            t,
                            epoch.epoch,
                            &parameters,
                            &field,
                            &camera.inner,
                            initial_robot_to_local_from_imu(imu, &feet.inner),
                        )?);
                    }
                    let l = active
                        .as_mut()
                        .ok_or_else(|| eyre!("localization not initialized"))?;
                    match m {
                        Measurement::Imu(t, imu) => {
                            l.ingest_imu(t, imu)?;
                        }
                        Measurement::Feet(f) => {
                            l.ingest_kinematics(*f)?;
                        }
                        Measurement::Visual(v) => {
                            let audit = v.clone();
                            let admitted = l.ingest_visual_localization_frame(v)?;
                            audit_frames.push((audit, admitted));
                        }
                        Measurement::Vo(v) => {
                            let previous = v
                                .delta
                                .as_ref()
                                .map(|d| camera_at(&cameras, d.previous_time))
                                .transpose()?;
                            let current = camera_at(&cameras, v.time)?;
                            l.ingest_visual_odometry(v, previous.as_ref(), Some(&current))?;
                        }
                    }
                }
                if let Some(l) = active.as_mut() {
                    let solved = l.solve(now);
                    let status = l.status();
                    let key = (status.epoch, status.generation, status.state);
                    if previous_status != Some(key) {
                        lifecycle.push(status);
                        previous_status = Some(key);
                    }
                    if let Some(e) = solved.estimate {
                        output.push((e, status.state));
                        estimates.insert(e.time, e);
                        estimates.retain(|t, _| t.saturating_add(Duration::from_secs(1)) >= now);
                    }
                }
                solve_us.push(timer.elapsed().as_secs_f64() * 1e6);
            }
        }
    }
    ensure!(!output.is_empty(), "replay produced no estimates");
    let field_estimates: Vec<_> = output
        .iter()
        .filter_map(|(estimate, _)| estimate.robot_to_field.map(|pose| (estimate, pose)))
        .collect();
    let transitions: Vec<_> = field_estimates
        .windows(2)
        .filter(|pair| pair[0].0.epoch == pair[1].0.epoch)
        .map(|pair| {
            (
                pair[1].0.time.abs_diff(pair[0].0.time).as_secs_f64(),
                (pair[1].1.pose.inner.translation.vector - pair[0].1.pose.inner.translation.vector)
                    .xy()
                    .norm(),
            )
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "field_dimensions": field, "wall_s": started.elapsed().as_secs_f64(),
            "association": timing(&mut association_us), "localization": timing(&mut solve_us),
            "field_estimates": field_estimates.len(),
            "longest_field_output_gap_s": transitions.iter().map(|(gap, _)| *gap).reduce(f64::max),
            "largest_position_step_m": transitions.iter().map(|(_, step)| *step).reduce(f64::max),
            "lifecycle": lifecycle, "audit_frames": audit_frames, "audit_estimates": output,
        }))?
    );
    Ok(())
}

fn timing(values: &mut [f64]) -> serde_json::Value {
    values.sort_by(f64::total_cmp);
    if values.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::json!({"count": values.len(), "total_s": values.iter().sum::<f64>() / 1e6,
        "median_ms": values[values.len() / 2] / 1000.0, "p95_ms": values[values.len() * 95 / 100] / 1000.0})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_required_topic_needs_messages() {
        let mut seen: BTreeSet<_> = REQUIRED_TOPICS.iter().map(|s| s.to_string()).collect();
        assert!(require_topics(&seen).is_ok());
        for topic in REQUIRED_TOPICS {
            seen.remove(*topic);
            assert!(
                require_topics(&seen)
                    .unwrap_err()
                    .to_string()
                    .contains(topic)
            );
            seen.insert(topic.to_string());
        }
    }

    #[test]
    fn camera_geometry_has_no_missing_data_fallback() {
        assert!(camera_at(&BTreeMap::new(), Time::from_nanos(1)).is_err());
    }
}
