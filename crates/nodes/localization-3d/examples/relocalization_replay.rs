//! Delivery-time replay of production association and localization, scored against
//! user-validated recorded poses. Usage: relocalization_replay recording.mcap [end_seconds]
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use booster::{ImuState, LowState};
use color_eyre::Result;
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
use ros_z::{
    message::{SerdeCdrCodec, WireDecoder},
    time::Time,
};
use types::{
    camera_geometry::{CameraGeometry, interpolate_camera_geometry},
    field_dimensions::FieldDimensions,
    localization::{LocalizationEstimate, LocalizationStatus},
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
    visual_localization::VisualLocalizationFrame,
    visual_odometry::VisualOdometer,
};

const CHECKPOINTS: &[f64] = &[
    162.236, 170.157, 238.487, 246.126, 284.751, 296.848, 347.541, 360.263, 397.793, 422.793,
];

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
) -> Option<CameraGeometry> {
    interpolate_camera_geometry(
        cache.range(..=t).next_back()?.1,
        cache.range(t..).next()?.1,
        t,
        Duration::from_millis(20),
    )
}
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(if std::env::args().nth(4).as_deref() == Some("quiet") {
            tracing::Level::WARN
        } else if std::env::args()
            .nth(3)
            .is_some_and(|mode| mode.starts_with("cost-"))
        {
            tracing::Level::TRACE
        } else {
            tracing::Level::DEBUG
        })
        .with_writer(std::io::stderr)
        .init();
    let path = std::env::args().nth(1).expect("recording path");
    let end: f64 = std::env::args()
        .nth(2)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(425.0);
    let mut file = File::open(path)?;
    let mut reader = LinearReader::new();
    let mut topics = BTreeMap::new();
    let mut events = Vec::new();
    let mut references = Vec::new();
    let mut recorded_visuals = Vec::new();
    let audit_reference = std::env::args().nth(3).as_deref() == Some("audit-reference");
    let checkpoint_rmse =
        std::env::args().nth(3).as_deref() == Some("checkpoint-rmse") || audit_reference;
    let mut start = u64::MAX;
    while let Some(event) = reader.next_event() {
        match event? {
            LinearReadEvent::ReadRequest(n) => {
                let n = file.read(reader.insert(n))?;
                if n == 0 {
                    break;
                }
                reader.notify_read(n);
            }
            LinearReadEvent::Record { opcode, data } => match mcap::parse_record(opcode, data)? {
                Record::Channel(c) => {
                    topics.insert(c.id, c.topic);
                }
                Record::Message { header, data } => {
                    start = start.min(header.log_time);
                    if header.log_time > start.saturating_add(((end + 2.0) * 1e9) as u64) {
                        continue;
                    }
                    let event = match topics.get(&header.channel_id).map(String::as_str) {
                        Some("field_mark_association/visual_localization_local")
                            if checkpoint_rmse =>
                        {
                            recorded_visuals.push(SerdeCdrCodec::<
                                TimeWrapper<VisualLocalizationFrame>,
                            >::deserialize(
                                &data
                            )?);
                            None
                        }
                        Some("inputs/low_state") => Some(Event::Imu(
                            Time::from_nanos(header.publish_time as i64),
                            SerdeCdrCodec::<LowState>::deserialize(&data)?.imu_state,
                        )),
                        Some("robot_kinematics") => Some(Event::Feet(Box::new(SerdeCdrCodec::<
                            TimeWrapper<RobotKinematics>,
                        >::deserialize(
                            &data
                        )?))),
                        Some("camera_geometry") => {
                            Some(Event::Camera(
                                SerdeCdrCodec::<TimeWrapper<CameraGeometry>>::deserialize(&data)?,
                            ))
                        }
                        Some("visual_odometry/current_left_camera_to_visual_odometer") => Some(
                            Event::Vo(SerdeCdrCodec::<VisualOdometer>::deserialize(&data)?),
                        ),
                        Some("detected_objects") => Some(Event::Objects(SerdeCdrCodec::<
                            TimeWrapper<Vec<Object<RobocupObjectLabel>>>,
                        >::deserialize(
                            &data
                        )?)),
                        Some("localization/status") => {
                            Some(Event::Reset(
                                SerdeCdrCodec::<LocalizationStatus>::deserialize(&data)?,
                            ))
                        }
                        Some("debug/solve_diagnostics") => Some(Event::Solve),
                        Some("localization/estimate") => {
                            references
                                .push(SerdeCdrCodec::<LocalizationEstimate>::deserialize(&data)?);
                            None
                        }
                        _ => None,
                    };
                    if let Some(event) = event {
                        events.push((header.log_time, event));
                    }
                }
                _ => (),
            },
        }
    }
    events.sort_by_key(|e| e.0);
    if audit_reference {
        let camera_samples: BTreeMap<_, _> = events
            .iter()
            .filter_map(|(_, event)| {
                if let Event::Camera(camera) = event {
                    Some((camera.time, camera.clone()))
                } else {
                    None
                }
            })
            .collect();
        let odometry: Vec<_> = events.iter().filter_map(|(_, event)| {
            let Event::Vo(sample) = event else { return None; };
            let delta = sample.delta.as_ref()?;
            let previous = camera_at(&camera_samples, delta.previous_time)?;
            let current = camera_at(&camera_samples, sample.time)?;
            let transform = previous.robot_to_camera.inner.inverse()
                * delta.current_left_camera_to_previous_left_camera
                * current.robot_to_camera.inner;
            Some(serde_json::json!({"previous_time":delta.previous_time,"time":sample.time,"epoch":sample.epoch,"transform":transform}))
        }).collect();
        println!(
            "{}",
            serde_json::to_string(
                &serde_json::json!({"origin_ns":start,"references":references,"frames":recorded_visuals,"odometry":odometry})
            )?
        );
        return Ok(());
    }
    if checkpoint_rmse {
        references.sort_by_key(|e| e.time);
        let mut rows = Vec::new();
        for &seconds in CHECKPOINTS {
            let time = Time::from_nanos(start as i64 + (seconds * 1e9) as i64);
            let Some(frame) = recorded_visuals
                .iter()
                .min_by_key(|f| f.time.abs_diff(time))
            else {
                color_eyre::eyre::bail!("recording contains no visual localization frames");
            };
            let after = references.partition_point(|e| e.time < frame.time);
            let b = references
                .get(after)
                .ok_or_else(|| color_eyre::eyre::eyre!("missing pose after exposure"))?;
            let a =
                if b.time == frame.time {
                    b
                } else {
                    references
                        .get(after.checked_sub(1).ok_or_else(|| {
                            color_eyre::eyre::eyre!("missing pose before exposure")
                        })?)
                        .unwrap()
                };
            let gap = b.time.abs_diff(a.time).as_secs_f64();
            let field = a.robot_to_field.zip(b.robot_to_field);
            let offset_ms = (frame.time.as_nanos() - time.as_nanos()) as f64 / 1e6;
            if a.epoch != b.epoch
                || a.generation != b.generation
                || a.epoch != frame.inner.epoch
                || a.generation != frame.inner.generation
                || field.is_none()
            {
                rows.push(serde_json::json!({"time":seconds,"unavailable":true,"frame_offset_ms":offset_ms,"pose_bracket_ms":gap*1000.}));
                continue;
            }
            let (pa, pb) = field.unwrap();
            let fraction = if gap == 0.0 {
                0.0
            } else {
                frame.time.duration_since(a.time).as_secs_f64() / gap
            };
            let pose = pa.pose.inner.lerp_slerp(&pb.pose.inner, fraction);
            let transform = frame.inner.robot_to_camera.inner.cast::<f64>() * pose.inverse();
            let errors = reprojection_errors(&frame.inner, &transform);
            let rmse = errors
                .as_ref()
                .filter(|e| !e.is_empty())
                .map(|e| (e.iter().map(|v| v * v).sum::<f64>() / e.len() as f64).sqrt());
            rows.push(serde_json::json!({"time":seconds,"frame_offset_ms":offset_ms,"pose_bracket_ms":gap*1000.,"timing_within_100ms":offset_ms.abs() <= 100.0 && gap <= 0.1,"features":frame.inner.associations.len(),"source":frame.inner.source,"rmse_px":rmse,"errors_px":errors}));
        }
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    if std::env::args().nth(3).as_deref() == Some("motion") {
        let mut odometry = Vec::new();
        let mut accelerations = Vec::new();
        for (_, event) in &events {
            match event {
                Event::Vo(v) => {
                    if let Some(delta) = &v.delta {
                        let dt = v.time.abs_diff(delta.previous_time).as_secs_f64();
                        if dt > 0.0 {
                            let pose = delta.current_left_camera_to_previous_left_camera;
                            odometry.push(serde_json::json!({"time":(v.time.as_nanos()-start as i64) as f64*1e-9,"dt":dt,"translation":pose.translation.vector.as_slice(),"speed":f64::from(pose.translation.vector.norm())/dt,"angle_deg":pose.rotation.angle().to_degrees()}));
                        }
                    }
                }
                Event::Imu(t, imu) => {
                    accelerations.push(serde_json::json!({"time":(t.as_nanos()-start as i64) as f64*1e-9,"norm":imu.linear_acceleration.inner.norm(),"force":imu.linear_acceleration.inner.as_slice()}));
                }
                _ => (),
            }
        }
        odometry.sort_by(|a, b| {
            b["speed"]
                .as_f64()
                .unwrap()
                .total_cmp(&a["speed"].as_f64().unwrap())
        });
        accelerations.sort_by(|a, b| {
            b["norm"]
                .as_f64()
                .unwrap()
                .total_cmp(&a["norm"].as_f64().unwrap())
        });
        odometry.truncate(30);
        accelerations.truncate(30);
        let headings = CHECKPOINTS.iter().filter_map(|&seconds| {
            let time = Time::from_nanos(start as i64 + (seconds * 1e9) as i64);
            let reference = references.iter().min_by_key(|r| r.time.abs_diff(time))?;
            let (stamp, imu) = events.iter().filter_map(|(_,event)| if let Event::Imu(t,imu) = event {Some((t,imu))} else {None})
                .min_by_key(|(t,_)| t.abs_diff(reference.time))?;
            let offset = reference.robot_to_field?.pose.inner.rotation.euler_angles().2 - f64::from(imu.roll_pitch_yaw.inner.z);
            Some(serde_json::json!({"time":seconds,"heading_offset_deg":offset.sin().atan2(offset.cos()).to_degrees(),"imu_age_ms":stamp.abs_diff(reference.time).as_secs_f64()*1000.0}))
        }).collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"odometry":odometry,"accelerations":accelerations,"reference_headings":headings})
            )?
        );
        return Ok(());
    }
    let mut parameters: Localization3dParameters = json5::from_str(include_str!(
        "../../../../etc/parameters/base/localization3d.json5"
    ))?;
    let mut association_parameters: FieldMarkAssociationParameters = json5::from_str(
        include_str!("../../../../etc/parameters/base/field_mark_association.json5"),
    )?;
    let mode = std::env::args().nth(3).unwrap_or_else(|| "baseline".into());
    match mode.as_str() {
        "baseline" | "cost-200" => (),
        "cost-100" => parameters.timing.trajectory_spacing = Duration::from_millis(100),
        "cost-50" => {
            parameters.timing.trajectory_spacing = Duration::from_millis(50);
            parameters.timing.preintegration_interval = Duration::from_millis(50);
            parameters.solver.step_tolerance = 0.0;
            parameters.solver.max_iterations = 60;
        }
        "convergent" => parameters.solver.max_iterations = 30,
        "no-accel" => parameters.accelerometer = None,
        "no-vo" => (),
        "fixed-heading" => parameters.max_heading_reference_drift_per_second = 0.0,
        "grid-100" => parameters.timing.trajectory_spacing = Duration::from_millis(100),
        "precise" | "grid-100-precise" | "grid-100-interval-50" | "grid-50-precise" => {
            parameters.solver.step_tolerance = 0.0;
            parameters.solver.max_iterations = 60;
            if mode != "precise" {
                parameters.timing.trajectory_spacing = Duration::from_millis(100);
            }
            if mode == "grid-100-interval-50" || mode == "grid-50-precise" {
                parameters.timing.preintegration_interval = Duration::from_millis(50);
            }
            if mode == "grid-50-precise" {
                parameters.timing.trajectory_spacing = Duration::from_millis(50);
            }
        }
        mode if mode.starts_with("iterations-") => {
            parameters.solver.max_iterations = mode["iterations-".len()..].parse()?;
        }
        "wide-heading" => parameters.max_heading_error = 89.0_f64.to_radians(),
        "strict-classes" => {
            association_parameters
                .global_localizer
                .class_mismatch_penalty = association_parameters.global_localizer.mahalanobis_gate
        }
        _ => color_eyre::eyre::bail!("unknown experiment: {mode}"),
    }
    // These are the landmark dimensions embedded in this recording, not the display preset.
    let field = FieldDimensions {
        length: 21.9,
        width: 13.9,
        goal_box_area_length: 1.9,
        penalty_area_length: 4.9,
        goal_post_diameter: 0.1,
        ..FieldDimensions::ITALIAN_OPEN
    };
    let mut active: Option<Localization> = None;
    let mut epoch = 0;
    let mut epoch_start = Time::from_nanos(start as i64);
    let mut cameras = BTreeMap::new();
    let mut attitudes = BTreeMap::new();
    let mut feet: Option<TimeWrapper<RobotKinematics>> = None;
    let mut pending = Vec::new();
    let mut estimates: BTreeMap<Time, LocalizationEstimate> = BTreeMap::new();
    let mut output = Vec::new();
    let mut audit_frames = Vec::new();
    let mut solve_us = Vec::new();
    let mut association_us = Vec::new();
    let mut association_count = 0;
    let mut previous_status = None;
    let mut lifecycle = Vec::new();
    let started = Instant::now();
    for (delivery, event) in events {
        let elapsed = (delivery - start) as f64 * 1e-9;
        if elapsed > end {
            break;
        }
        let now = Time::from_nanos(delivery as i64);
        match event {
            Event::Reset(s) if s.epoch != epoch => {
                epoch = s.epoch;
                epoch_start = s.time;
                active = None;
                pending.clear();
                estimates.clear();
                eprintln!("reset {elapsed:.3} epoch {epoch}");
            }
            Event::Reset(_) => (),
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
            Event::Vo(v) => {
                if mode != "no-vo" {
                    pending.push(Measurement::Vo(v));
                }
            }
            Event::Objects(objects) => {
                let Some(localization) = active.as_mut() else {
                    continue;
                };
                localization.advance_time(now);
                let status = localization.status();
                let t = objects.time;
                if t < epoch_start || t.saturating_add(Duration::from_millis(250)) < now {
                    continue;
                }
                let Some(camera) = camera_at(&cameras, t) else {
                    continue;
                };
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
                    });
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
                        attitude,
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
                    association_count += 1;
                    pending.push(Measurement::Visual(TimeWrapper {
                        time: t,
                        inner: VisualLocalizationFrame {
                            epoch,
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
                let timer = Instant::now();
                pending.sort_by_key(Measurement::time);
                for m in pending.drain(..) {
                    let t = m.time();
                    if t < epoch_start || t > now {
                        continue;
                    }
                    if active.is_none() {
                        let Measurement::Imu(_, ref imu) = m else {
                            continue;
                        };
                        let Some((_, camera)) = cameras
                            .range(..=t)
                            .next_back()
                            .filter(|(stamp, _)| stamp.abs_diff(t) <= Duration::from_millis(20))
                        else {
                            continue;
                        };
                        let Some(ref feet) = feet else { continue };
                        active = Some(Localization::new(
                            t,
                            epoch,
                            &parameters,
                            &field,
                            &camera.inner,
                            initial_robot_to_local_from_imu(imu, &feet.inner),
                        )?);
                    }
                    let l = active.as_mut().unwrap();
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
                                .and_then(|d| camera_at(&cameras, d.previous_time));
                            let current = camera_at(&cameras, v.time);
                            l.ingest_visual_odometry(v, previous.as_ref(), current.as_ref())?;
                        }
                    }
                }
                if let Some(l) = active.as_mut() {
                    let solved = l.solve(now);
                    let status = l.status();
                    let key = (status.epoch, status.generation, status.state);
                    if previous_status != Some(key) {
                        eprintln!("state {elapsed:.3} {key:?}");
                        lifecycle.push(serde_json::json!({"time":elapsed,"epoch":status.epoch,"generation":status.generation,"state":status.state,"heading_offset_deg":status.heading.map(|h|h.imu_to_field.inner.angle().to_degrees()),"pose":solved.estimate.as_ref().and_then(|e| e.robot_to_field).map(|p| [p.pose.inner.translation.x,p.pose.inner.translation.y,p.pose.inner.translation.z,p.pose.inner.rotation.euler_angles().2])}));
                        previous_status = Some(key);
                    }
                    if let Some(e) = solved.estimate {
                        output.push((e.clone(), status.state));
                        estimates.insert(e.time, e);
                        estimates.retain(|t, _| t.saturating_add(Duration::from_secs(1)) >= now);
                    }
                }
                solve_us.push(timer.elapsed().as_secs_f64() * 1e6);
            }
        }
    }
    let mut checkpoints = Vec::new();
    for &seconds in CHECKPOINTS.iter().filter(|s| **s <= end) {
        let time = Time::from_nanos(start as i64 + (seconds * 1e9) as i64);
        let reference = references
            .iter()
            .min_by_key(|e| e.time.abs_diff(time))
            .unwrap();
        let estimate = output.iter().min_by_key(|(e, _)| e.time.abs_diff(time));
        let result = estimate.and_then(|(e, state)| e.robot_to_field.zip(reference.robot_to_field).map(|(p,r)| {
            let delta = p.pose.inner.translation.vector-r.pose.inner.translation.vector;
            let yaw = p.pose.inner.rotation.euler_angles().2-r.pose.inner.rotation.euler_angles().2;
            let yaw_error = yaw.sin().atan2(yaw.cos()).to_degrees();
            let pass = delta.xy().norm() <= 1.0 && delta.z.abs() <= 0.5 && yaw_error.abs() <= 15.0 && e.time.abs_diff(time) <= Duration::from_millis(100) && *state == types::localization::LocalizationState::Tracking;
            serde_json::json!({"time":seconds,"pass":pass,"xy_error_m":delta.xy().norm(),"height_error_m":delta.z,"yaw_error_deg":yaw_error,"state":state,"age_ms":e.time.abs_diff(time).as_secs_f64()*1000.,"pose":p.pose.inner.translation.vector.as_slice(),"reference":r.pose.inner.translation.vector.as_slice()})
        })).unwrap_or_else(|| serde_json::json!({"time":seconds,"missing":true}));
        checkpoints.push(result);
    }
    fn timing(v: &mut [f64]) -> serde_json::Value {
        v.sort_by(f64::total_cmp);
        if v.is_empty() {
            return serde_json::Value::Null;
        }
        serde_json::json!({"count":v.len(),"total_s":v.iter().sum::<f64>()/1e6,"median_ms":v[v.len()/2]/1000.,"p95_ms":v[v.len()*95/100]/1000.})
    }
    let mut transitions = output
        .iter()
        .filter(|(e, _)| e.robot_to_field.is_some())
        .collect::<Vec<_>>()
        .windows(2)
        .filter_map(|pair| {
            let ((a, sa), (b, sb)) = (pair[0], pair[1]);
            if a.epoch != b.epoch { return None; }
            let (pa, pb) = (a.robot_to_field?.pose.inner, b.robot_to_field?.pose.inner);
            let delta = pb.translation.vector - pa.translation.vector;
            let yaw = pb.rotation.euler_angles().2 - pa.rotation.euler_angles().2;
            Some(serde_json::json!({
                "from_s": (a.time.as_nanos() - start as i64) as f64 * 1e-9,
                "to_s": (b.time.as_nanos() - start as i64) as f64 * 1e-9,
                "dt_s": b.time.abs_diff(a.time).as_secs_f64(),
                "xy_m": delta.xy().norm(), "height_m": delta.z,
                "yaw_deg": yaw.sin().atan2(yaw.cos()).to_degrees(),
                "from": pa.translation.vector.as_slice(), "to": pb.translation.vector.as_slice(),
                "from_position_sigma_m": a.robot_to_field?.covariance.fixed_view::<3,3>(3,3).trace().max(0.0).sqrt(),
                "to_position_sigma_m": b.robot_to_field?.covariance.fixed_view::<3,3>(3,3).trace().max(0.0).sqrt(),
                "states": [sa, sb], "generations": [a.generation, b.generation]
            }))
        })
        .collect::<Vec<_>>();
    let jumps = transitions
        .iter()
        .filter(|t| t["dt_s"].as_f64().unwrap() < 0.5 && t["xy_m"].as_f64().unwrap() > 1.0)
        .count();
    let mut longest_field_output_gaps = transitions.clone();
    longest_field_output_gaps.sort_by(|a, b| {
        b["dt_s"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["dt_s"].as_f64().unwrap())
    });
    longest_field_output_gaps.truncate(30);
    transitions.sort_by(|a, b| {
        b["xy_m"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["xy_m"].as_f64().unwrap())
    });
    transitions.truncate(30);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"mode":mode,"wall_s":started.elapsed().as_secs_f64(),"association":timing(&mut association_us),"localization":timing(&mut solve_us),"association_frames":association_count,"checkpoints":checkpoints,"lifecycle":lifecycle,"jumps_over_1m":jumps,"largest_transitions":transitions,"longest_field_output_gaps":longest_field_output_gaps,"field_estimates":output.iter().filter(|(e,_)| e.robot_to_field.is_some()).count(),"tracking_estimates":output.iter().filter(|(e,s)| e.robot_to_field.is_some() && *s == types::localization::LocalizationState::Tracking).count(),"audit_frames":audit_frames,"audit_estimates":output})
        )?
    );
    Ok(())
}

fn reprojection_errors(
    frame: &VisualLocalizationFrame,
    field_to_camera: &nalgebra::Isometry3<f64>,
) -> Option<Vec<f64>> {
    frame
        .associations
        .iter()
        .map(|observation| {
            let p = field_to_camera * observation.field_point.inner.cast::<f64>();
            if p.z <= 0.01 {
                return None;
            }
            let intrinsic = &frame.camera_intrinsic;
            let pixel = nalgebra::Vector2::new(
                f64::from(intrinsic.focals.x) * p.x / p.z + f64::from(intrinsic.optical_center.x()),
                f64::from(intrinsic.focals.y) * p.y / p.z + f64::from(intrinsic.optical_center.y()),
            );
            let error = (pixel - observation.detection.inner.coords.cast::<f64>()).norm();
            error.is_finite().then_some(error)
        })
        .collect()
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    use linear_algebra::{IntoTransform, point};

    #[test]
    fn reprojection_reports_pixel_distance_and_rejects_points_behind_camera() {
        let mut frame = VisualLocalizationFrame {
            epoch: 0,
            generation: 0,
            source: Default::default(),
            robot_to_camera: nalgebra::Isometry3::identity().framed_transform(),
            camera_intrinsic: projection::intrinsic::Intrinsic::new(
                nalgebra::vector![100., 200.],
                point![320., 240.],
            ),
            associations: vec![types::visual_localization::FieldMarkAssociation {
                field_point: point![1., 2., 10.],
                detection: point![333., 284.],
            }],
        };
        assert_eq!(
            reprojection_errors(&frame, &nalgebra::Isometry3::identity()),
            Some(vec![5.])
        );
        frame.associations[0].field_point.inner.z = -10.;
        assert!(reprojection_errors(&frame, &nalgebra::Isometry3::identity()).is_none());
    }
}
