//! Isolated calibration experiment, including VO ablation, on a confirmed stationary
//! recording. No stationary/contact equalities or measured bias are given to the graph.
//! Usage: imu_calibration_replay recording.mcap [duration_seconds (30)] [mode (all)]
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use booster::{JointsMotorState, LowState};
use kinematics::{
    forward::{left_sole_to_robot, right_sole_to_robot},
    robot_kinematics::RobotKinematics,
};
use linear_algebra::Isometry3;
use localization_3d::{ImuBiasParameters, Localization, Localization3dParameters};
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
    localization::LocalizationEstimate,
    time_wrapper::TimeWrapper,
    visual_odometry::VisualOdometer,
};

enum Input {
    Low(Time, LowState),
    Vo(VisualOdometer),
}

fn main() -> color_eyre::Result<()> {
    let path = std::env::args_os().nth(1).expect("MCAP path");
    let duration: f64 = std::env::args()
        .nth(2)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(30.0);
    let selected = std::env::args().nth(3).unwrap_or_else(|| "all".into());
    color_eyre::eyre::ensure!(
        [
            "all",
            "fixed-zero",
            "learn-with-vo",
            "learn-vo-outage",
            "learn-without-vo"
        ]
        .contains(&selected.as_str()),
        "unknown mode"
    );
    color_eyre::eyre::ensure!(
        duration.is_finite() && duration > 5.0,
        "duration must exceed 5 s"
    );
    let duration_ns: u64 = Duration::try_from_secs_f64(duration)?
        .as_nanos()
        .try_into()?;
    let mut file = File::open(path)?;
    let mut reader = LinearReader::new();
    let mut topics = BTreeMap::new();
    let mut cameras = BTreeMap::new();
    let mut inputs = Vec::new();
    let mut first = None;
    let mut initial = None;
    // Linear reading also supports a copied, still-open recording without a footer.
    'read: while let Some(event) = reader.next_event() {
        match event? {
            LinearReadEvent::ReadRequest(count) => {
                let n = file.read(reader.insert(count))?;
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
                    let start = *first.get_or_insert(header.log_time);
                    if header.log_time.saturating_sub(start) > duration_ns {
                        break 'read;
                    }
                    match topics.get(&header.channel_id).map(String::as_str) {
                        Some("inputs/low_state") => inputs.push((
                            header.log_time,
                            Input::Low(
                                Time::from_nanos(header.publish_time as i64),
                                SerdeCdrCodec::<LowState>::deserialize(&data)?,
                            ),
                        )),
                        Some("camera_geometry") => {
                            let c =
                                SerdeCdrCodec::<TimeWrapper<CameraGeometry>>::deserialize(&data)?;
                            cameras.insert(c.time, c);
                        }
                        Some("localization/estimate") if initial.is_none() => {
                            initial =
                                Some(SerdeCdrCodec::<LocalizationEstimate>::deserialize(&data)?)
                        }
                        Some("visual_odometry/current_left_camera_to_visual_odometer") => inputs
                            .push((
                                header.log_time,
                                Input::Vo(SerdeCdrCodec::<VisualOdometer>::deserialize(&data)?),
                            )),
                        _ => {}
                    }
                }
                _ => {}
            },
        }
    }
    inputs.sort_by_key(|(delivery, _)| *delivery);
    let parameters: Localization3dParameters = json5::from_str(include_str!(
        "../../../../etc/parameters/base/localization3d.json5"
    ))?;
    let camera_at = |time| {
        let (_, a) = cameras.range(..=time).next_back()?;
        let (_, b) = cameras.range(time..).next()?;
        interpolate_camera_geometry(a, b, time, parameters.timing.max_camera_gap)
    };
    let initial = initial.ok_or_else(|| color_eyre::eyre::eyre!("missing initial estimate"))?;
    let origin = initial.time;
    let camera =
        camera_at(origin).ok_or_else(|| color_eyre::eyre::eyre!("missing camera bracket"))?;
    let mut reference = nalgebra::Vector3::<f64>::zeros();
    let mut count = 0;
    for (_, input) in &inputs {
        if let Input::Low(time, low) = input {
            if *time < origin || *time > origin + Duration::from_secs(5) {
                continue;
            }
            let rpy = low.imu_state.roll_pitch_yaw.inner.cast::<f64>();
            let rotation = nalgebra::UnitQuaternion::from_euler_angles(rpy.x, rpy.y, rpy.z);
            reference += low.imu_state.linear_acceleration.inner.cast::<f64>()
                - rotation.inverse() * nalgebra::vector![0.0, 0.0, 9.81];
            count += 1;
        }
    }
    color_eyre::eyre::ensure!(count > 0, "no initial IMU samples");
    reference /= count as f64;
    println!(
        "{}",
        serde_json::json!({"stationary_reference_only":reference,"samples":count})
    );
    for mode in [
        "fixed-zero",
        "learn-with-vo",
        "learn-vo-outage",
        "learn-without-vo",
    ] {
        if selected != "all" && selected != mode {
            continue;
        }
        let mut p = parameters.clone();
        if mode == "fixed-zero" {
            p.imu_bias = ImuBiasParameters {
                accelerometer_initial_sigma: 1e-5,
                gyroscope_initial_sigma: 1e-5,
                accelerometer_random_walk: 1e-7,
                gyroscope_random_walk: 1e-7,
            };
        }
        let initial_pose = initial.robot_to_local.pose.inner;
        let mut localization = Localization::new(
            origin,
            initial.epoch,
            &p,
            &FieldDimensions::SPL_2025,
            &camera,
            Isometry3::wrap(initial_pose.cast()),
        )?;
        let mut next = origin;
        let mut last = initial_pose;
        let mut max_displacement = 0.0_f64;
        let mut max_after_learning = 0.0_f64;
        let mut converged = 0;
        let mut terminations = BTreeMap::<String, usize>::new();
        let mut estimates = 0;
        let mut cycles = Vec::new();
        let mut pending = Duration::ZERO;
        let mut bias = None;
        let mut last_report = 0;
        for (delivery, input) in &inputs {
            let now = Time::from_nanos(*delivery as i64);
            if now < origin {
                continue;
            }
            let elapsed = now.duration_since(origin).as_secs_f64();
            let with_vo =
                mode != "learn-without-vo" && (mode != "learn-vo-outage" || elapsed < 20.0);
            let started = Instant::now();
            match input {
                Input::Low(time, low) if *time >= origin => {
                    localization.ingest_imu(*time, low.imu_state)?;
                    let angles = low.serial_motor_states()?.positions();
                    let mut feet = RobotKinematics::default();
                    feet.left_leg.sole_to_robot = left_sole_to_robot(&angles.left_leg);
                    feet.right_leg.sole_to_robot = right_sole_to_robot(&angles.right_leg);
                    localization.ingest_kinematics(TimeWrapper {
                        time: *time,
                        inner: feet,
                    })?;
                }
                Input::Vo(vo) if with_vo => {
                    let prev = vo.delta.as_ref().and_then(|d| camera_at(d.previous_time));
                    let curr = camera_at(vo.time);
                    localization.ingest_visual_odometry(
                        vo.clone(),
                        prev.as_ref(),
                        curr.as_ref(),
                    )?;
                }
                _ => {}
            }
            pending += started.elapsed();
            if now < next {
                continue;
            }
            next = now + Duration::from_millis(50);
            let solved = localization.solve(now);
            cycles.push(pending + solved.diagnostics.estimation_duration);
            pending = Duration::ZERO;
            converged += usize::from(solved.diagnostics.termination == "GradientTolerance");
            *terminations
                .entry(solved.diagnostics.termination.clone())
                .or_default() += 1;
            if let Some(value) = solved.diagnostics.imu_bias {
                bias = Some(value);
            }
            if let Some(e) = solved.estimate {
                estimates += 1;
                last = e.robot_to_local.pose.inner;
                let displacement =
                    (last.translation.vector - initial_pose.translation.vector).norm();
                max_displacement = max_displacement.max(displacement);
                if elapsed >= 10.0 {
                    max_after_learning = max_after_learning.max(displacement);
                }
            }
            if elapsed as u64 / 5 > last_report {
                last_report = elapsed as u64 / 5;
                println!(
                    "{}",
                    serde_json::json!({"mode":mode,"seconds":elapsed,"bias":bias,"translation":last.translation.vector})
                );
            }
        }
        let total: Duration = cycles.iter().sum();
        cycles.sort_unstable();
        println!(
            "{}",
            serde_json::json!({"mode":mode,"cycles":cycles.len(),"estimates":estimates,"converged":converged,"terminations":terminations,
            "compute_s":total.as_secs_f64(),"p50_ms":cycles[cycles.len()/2].as_secs_f64()*1000.0,
            "p95_ms":cycles[(cycles.len()*95/100).min(cycles.len()-1)].as_secs_f64()*1000.0,
            "max_displacement_m":max_displacement,"max_after_10s_m":max_after_learning,"final_translation":last.translation.vector,"bias":bias})
        );
    }
    Ok(())
}
