//! Opt-in numerical regression for the 2026-09-26 divergence window. Replays
//! recorded IMU/VO delivery order and the suspect recovery, not robot behavior.
use super::*;
use linear_algebra::{Orientation2, Orientation3};
use mcap::{
    records::Record,
    sans_io::{LinearReadEvent, LinearReader},
};
use ros_z::message::{SerdeCdrCodec, WireDecoder};
use std::{collections::BTreeMap, fs::File, io::Read, time::Duration};
use types::camera_geometry::interpolate_camera_geometry;

enum Input {
    Imu(Time, ImuState),
    Odometry(VisualOdometer),
    Visual(TimeWrapper<VisualLocalizationFrame>),
}

#[test]
#[ignore = "requires the 2026-09-26 recovered.mcap recording"]
fn recorded_divergence_window_remains_bounded_without_contact() {
    const START: i64 = 1_790_430_115_057_029_550 + 499_000_000_000;
    const END: i64 = START + 17_000_000_000;
    let path = std::env::var_os("HULK_RECOVERY_MCAP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../recovered.mcap")
        });
    let mut file = File::open(&path).unwrap();
    let mut reader = LinearReader::new();
    let mut topics = BTreeMap::new();
    let mut inputs = Vec::new();
    let mut cameras = BTreeMap::new();
    let mut reference = None;
    let mut initial = None;
    while let Some(event) = reader.next_event() {
        match event.unwrap() {
            LinearReadEvent::ReadRequest(count) => {
                let count = file.read(reader.insert(count)).unwrap();
                reader.notify_read(count);
            }
            LinearReadEvent::Record { opcode, data } => match mcap::parse_record(opcode, data)
                .unwrap()
            {
                Record::Channel(channel) => {
                    topics.insert(channel.id, channel.topic);
                }
                Record::Message { header, data } => {
                    let delivery = header.log_time as i64;
                    match topics.get(&header.channel_id).map(String::as_str) {
                        Some("localization/status") if delivery <= START => {
                            let status =
                                SerdeCdrCodec::<LocalizationStatus>::deserialize(&data).unwrap();
                            reference = status.heading;
                        }
                        Some("localization/estimate") if delivery <= START => {
                            initial = Some(
                                SerdeCdrCodec::<LocalizationEstimate>::deserialize(&data).unwrap(),
                            );
                        }
                        Some("inputs/low_state") if (START..=END).contains(&delivery) => {
                            let low =
                                SerdeCdrCodec::<booster::LowState>::deserialize(&data).unwrap();
                            inputs.push((
                                delivery,
                                Input::Imu(
                                    Time::from_nanos(header.publish_time as i64),
                                    low.imu_state,
                                ),
                            ));
                        }
                        Some("camera_geometry")
                            if (START - 1_000_000_000..=END).contains(&delivery) =>
                        {
                            let camera =
                                SerdeCdrCodec::<TimeWrapper<CameraGeometry>>::deserialize(&data)
                                    .unwrap();
                            cameras.insert(camera.time, camera);
                        }
                        Some("visual_odometry/current_left_camera_to_visual_odometer")
                            if (START..=END).contains(&delivery) =>
                        {
                            inputs.push((
                                delivery,
                                Input::Odometry(
                                    SerdeCdrCodec::<VisualOdometer>::deserialize(&data).unwrap(),
                                ),
                            ));
                        }
                        Some("field_mark_association/visual_localization_local")
                            if header.sequence == 3722 && (START..=END).contains(&delivery) =>
                        {
                            inputs.push((delivery, Input::Visual(SerdeCdrCodec::<TimeWrapper<VisualLocalizationFrame>>::deserialize(&data).unwrap())));
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
        }
    }
    inputs.sort_by_key(|(delivery, _)| *delivery);
    let initial = initial.expect("pre-incident pose");
    let reference = reference.expect("pre-incident heading reference");
    let parameters: Localization3dParameters = json5::from_str(include_str!(
        "../../../../../etc/parameters/base/localization3d.json5"
    ))
    .unwrap();
    let camera_at = |time: Time| -> Option<CameraGeometry> {
        let (_, before) = cameras.range(..=time).next_back()?;
        let (_, after) = cameras.range(time..).next()?;
        interpolate_camera_geometry(before, after, time, parameters.timing.max_camera_gap)
    };
    let (origin, first_imu) = inputs
        .iter()
        .find_map(|(_, input)| match input {
            Input::Imu(time, imu) => Some((*time, *imu)),
            _ => None,
        })
        .unwrap();
    let mut field = FieldDimensions::SPL_2025;
    field.length = 8.92;
    field.width = 5.94;
    let mut localization = Localization::new(
        origin,
        initial.epoch,
        &parameters,
        &field,
        &camera_at(origin).unwrap(),
        Isometry3::wrap(initial.robot_to_local.pose.inner.cast()),
    )
    .unwrap();
    let rpy = first_imu.roll_pitch_yaw.inner.cast::<f64>();
    localization.heading_reference = Some(HeadingReference::new(
        reference.time,
        reference
            .at(origin, Orientation2::new(rpy.z))
            .unwrap()
            .expected,
        Orientation3::from_euler_angles(rpy.x, rpy.y, rpy.z),
    ));
    localization.status.state = LocalizationState::LostTrack;
    localization.status.time = origin;
    let mut next_solve = Time::from_nanos(START);
    let mut estimates = 0;
    let mut solves = 0;
    let mut recovery_frames = 0;
    let mut max_height = 0.0_f64;
    let mut rebuilt = 0;
    let mut field_estimates = 0;
    let mut cycles = Vec::new();
    let mut ingestion = Duration::ZERO;
    let mut total_ingestion = Duration::ZERO;
    let mut failures = 0;
    for (delivery, input) in inputs {
        let started = std::time::Instant::now();
        match input {
            Input::Imu(time, imu) => {
                localization.ingest_imu(time, imu).unwrap();
            }
            Input::Odometry(vo) => {
                let previous = vo
                    .delta
                    .as_ref()
                    .and_then(|delta| camera_at(delta.previous_time));
                let current = camera_at(vo.time);
                localization
                    .ingest_visual_odometry(vo, previous.as_ref(), current.as_ref())
                    .unwrap();
            }
            Input::Visual(mut frame) => {
                frame.inner.generation = localization.status().generation;
                assert!(
                    localization
                        .ingest_visual_localization_frame(frame)
                        .unwrap()
                );
                recovery_frames += 1;
            }
        }
        let elapsed = started.elapsed();
        ingestion += elapsed;
        total_ingestion += elapsed;
        let now = Time::from_nanos(delivery);
        if now < next_solve {
            continue;
        }
        next_solve = now + Duration::from_millis(50);
        let solved = localization.solve(now);
        cycles.push(ingestion + solved.diagnostics.estimation_duration);
        ingestion = Duration::ZERO;
        failures += usize::from(solved.diagnostics.failure.is_some());
        solves += 1;
        rebuilt += usize::from(solved.diagnostics.motion_rebuilt);
        if let Some(estimate) = solved.estimate {
            field_estimates += usize::from(estimate.robot_to_field.is_some());
            let pose = estimate.robot_to_local.pose;
            assert!(pose.inner.to_homogeneous().iter().all(|v| v.is_finite()));
            max_height = max_height.max(pose.translation().z().abs());
            estimates += 1;
        }
    }
    assert_eq!(recovery_frames, 1);
    eprintln!(
        "MCAP 08:19–08:36: {estimates}/{solves} motion estimates ({field_estimates} field estimates), max |height|={max_height:.3} m, {rebuilt} field resets"
    );
    report_timing("recorded-local", cycles, 17.0, total_ingestion, failures);
    assert!(max_height < 3.0, "max |height|={max_height}");
    assert!(
        estimates > solves / 2,
        "{estimates}/{solves} usable motion outputs"
    );
}

pub(super) fn report_timing(
    name: &str,
    mut cycles: Vec<Duration>,
    sensor_seconds: f64,
    ingestion: Duration,
    failures: usize,
) {
    let total: Duration = cycles.iter().sum();
    cycles.sort_unstable();
    let percentile = |p: f64| {
        cycles[((cycles.len() as f64 * p).ceil() as usize).saturating_sub(1)].as_secs_f64() * 1000.0
    };
    eprintln!(
        "BENCH {name}: cycles={} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} total_s={:.3} ingestion_s={:.3} compute_per_sensor={:.3} failures={failures}",
        cycles.len(),
        percentile(0.5),
        percentile(0.95),
        percentile(0.99),
        percentile(1.0),
        total.as_secs_f64(),
        ingestion.as_secs_f64(),
        total.as_secs_f64() / sensor_seconds
    );
}
