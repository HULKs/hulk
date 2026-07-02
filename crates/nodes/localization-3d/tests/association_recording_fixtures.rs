use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use coordinate_systems::{Camera, Field, Pixel, Robot};
use field_mark_association::{
    DetectedVisualFeature, GlobalLocalizationDebugStatus, GlobalLocalizerParameters,
    find_detected_visual_features, localize_global_visual_features,
};
use linear_algebra::{Isometry3, Point2};
use mcap::MessageStream;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    object_detection::{Object, RobocupObjectLabel},
};

#[path = "support/recording_decode.rs"]
mod recording_decode;

use recording_decode::{decode_recorded_camera_matrix, decode_recorded_message};

const FIXTURE_JSON: &str = include_str!("association_fixtures.json");
const CAMERA_MATRIX_MAX_AGE: Duration = Duration::from_millis(100);
const DEBUG_MAX_AGE: Duration = Duration::from_millis(100);
const EXPECTED_PIXEL_GATE: f32 = 80.0;
const MAX_ACCEPTED_FIXTURES: usize = 6;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AssociationFixture {
    name: String,
    camera_matrix: CameraMatrix,
    detections: Vec<Object<RobocupObjectLabel>>,
    expected: Vec<ExpectedAssociation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExpectedAssociation {
    detection: [f32; 2],
    landmark: [f32; 2],
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct RecordedGlobalLocalizationDebug {
    robot_to_field: Isometry3<Robot, Field>,
    status: GlobalLocalizationDebugStatus,
    inliers: usize,
    reprojection_rmse: f32,
    total_cost: f32,
}

#[test]
fn real_recording_association_fixtures_match_expected_landmarks() -> Result<(), Box<dyn Error>> {
    let fixtures: Vec<AssociationFixture> = serde_json::from_str(FIXTURE_JSON)?;
    if fixtures.is_empty() {
        return Err("association fixture file is empty".into());
    }

    for fixture in fixtures {
        let features = find_detected_visual_features(&fixture.detections);
        let localization = localize_global_visual_features(
            &features,
            &fixture.camera_matrix,
            &FieldDimensions::SPL_2025,
            None,
            &GlobalLocalizerParameters::default(),
        );
        let actual = localization
            .associations
            .into_iter()
            .map(|association| {
                AssociationKey::from_points(association.detection, association.field_point.xy())
            })
            .collect::<Vec<_>>();
        let expected = fixture
            .expected
            .iter()
            .map(|association| {
                AssociationKey::from_arrays(association.detection, association.landmark)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            canonical_mod_symmetry(actual),
            canonical_mod_symmetry(expected),
            "{}",
            fixture.name
        );
    }

    Ok(())
}

#[test]
#[ignore = "extracts compact fixtures from LOCALIZATION_3D_ASSOCIATION_RECORDING"]
// Regenerate with:
// LOCALIZATION_3D_ASSOCIATION_RECORDING=/path/to/recording.mcap \
// cargo test -p localization-3d --test association_recording_fixtures \
// extract_association_fixtures_from_recording -- --ignored --nocapture
fn extract_association_fixtures_from_recording() -> Result<(), Box<dyn Error>> {
    let recording_path = std::env::var("LOCALIZATION_3D_ASSOCIATION_RECORDING").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "set LOCALIZATION_3D_ASSOCIATION_RECORDING to the MCAP recording path",
        )
    })?;
    let fixtures = extract_fixtures(Path::new(&recording_path))?;
    if fixtures.is_empty() {
        return Err("no matching association fixtures found in recording".into());
    }

    fs::write(fixture_path(), serde_json::to_string_pretty(&fixtures)?)?;
    Ok(())
}

fn extract_fixtures(path: &Path) -> Result<Vec<AssociationFixture>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut cameras = Vec::new();
    let mut detections = Vec::new();
    let mut debugs = Vec::new();
    let mut stats = ExtractionStats::default();

    for (order, message) in MessageStream::new(&bytes)?.enumerate() {
        let message = message?;
        let log_time = system_time_from_nanos(message.log_time);
        match message.channel.topic.as_str() {
            "camera_matrix" => {
                stats.camera_messages += 1;
                let camera_matrix = decode_recorded_camera_matrix(&message)?;
                cameras.push(Timed {
                    log_time,
                    source_time: camera_matrix.time.to_wallclock(),
                    value: camera_matrix.inner,
                });
            }
            "detected_objects" => {
                stats.detection_messages += 1;
                let objects: Vec<Object<RobocupObjectLabel>> = decode_recorded_message(&message)?;
                detections.push(Timed {
                    log_time,
                    source_time: system_time_from_nanos(message.publish_time),
                    value: objects,
                });
            }
            "debug/global_localization" => {
                stats.debug_messages += 1;
                let debug: Option<RecordedGlobalLocalizationDebug> =
                    decode_recorded_message(&message)?;
                debugs.push(Timed {
                    log_time,
                    source_time: system_time_from_nanos(message.publish_time),
                    value: debug,
                });
            }
            _ => {
                let _ = order;
            }
        }
    }

    let mut fixtures = Vec::new();
    for detection in detections {
        stats.frames_considered += 1;
        if fixtures.len() >= MAX_ACCEPTED_FIXTURES {
            break;
        }
        let Some(camera) = nearest_by_time(&cameras, detection.source_time) else {
            stats.missing_camera += 1;
            continue;
        };
        if abs_duration(camera.source_time, detection.source_time) > CAMERA_MATRIX_MAX_AGE {
            stats.stale_camera += 1;
            continue;
        }
        let Some(debug) = nearest_log_time(&debugs, detection.log_time) else {
            stats.missing_debug += 1;
            continue;
        };
        if abs_duration(debug.log_time, detection.log_time) > DEBUG_MAX_AGE {
            stats.stale_debug += 1;
            continue;
        }
        let Some(debug_value) = debug.value.as_ref() else {
            stats.empty_debug += 1;
            continue;
        };
        if !matches!(
            debug_value.status,
            GlobalLocalizationDebugStatus::UniqueModuloSymmetry
        ) {
            stats.non_unique_debug += 1;
        }

        let expected =
            expected_associations_from_debug(&detection.value, &camera.value, &debug_value);
        if expected.len() != debug_value.inliers {
            stats.expected_count_mismatch += 1;
            continue;
        }
        if expected.len() < 3 {
            stats.too_few_expected += 1;
            continue;
        }
        if !new_solver_matches(&detection.value, &camera.value, &expected) {
            stats.solver_mismatch += 1;
            continue;
        }

        stats.accepted += 1;
        fixtures.push(AssociationFixture {
            name: format!("recording_frame_{}", fixtures.len()),
            camera_matrix: camera.value.clone(),
            detections: detection.value,
            expected,
        });
    }

    eprintln!("association fixture extraction stats: {stats:#?}");

    Ok(fixtures)
}

fn expected_associations_from_debug(
    objects: &[Object<RobocupObjectLabel>],
    camera_matrix: &CameraMatrix,
    debug: &RecordedGlobalLocalizationDebug,
) -> Vec<ExpectedAssociation> {
    let detections = feature_detections(objects);
    let landmarks = field_landmarks(&FieldDimensions::SPL_2025);
    let field_to_camera = robot_to_camera(camera_matrix) * debug.robot_to_field.inverse();
    let mut candidates = Vec::new();

    for (detection_index, detection) in detections.iter().enumerate() {
        let mut best = None;
        for landmark in landmarks
            .iter()
            .filter(|landmark| landmark.label == detection.label)
        {
            let Some(projected) =
                project_landmark(field_to_camera, camera_matrix.intrinsics, landmark.xy)
            else {
                continue;
            };
            let residual = (projected - detection.feature.pixel).inner.norm();
            if residual < EXPECTED_PIXEL_GATE
                && best
                    .as_ref()
                    .is_none_or(|(_, best_residual): &(usize, f32)| residual < *best_residual)
            {
                best = Some((landmark.id, residual));
            }
        }
        if let Some((landmark_id, residual)) = best {
            candidates.push(ExpectedCandidate {
                detection_index,
                landmark_id,
                residual,
            });
        }
    }

    candidates.sort_by(|left, right| left.residual.total_cmp(&right.residual));
    let mut used_detections = vec![false; detections.len()];
    let mut used_landmarks = vec![false; landmarks.len()];
    let mut expected = Vec::new();
    for candidate in candidates {
        if used_detections[candidate.detection_index]
            || used_landmarks
                .get(candidate.landmark_id)
                .copied()
                .unwrap_or(true)
        {
            continue;
        }
        used_detections[candidate.detection_index] = true;
        if let Some(used) = used_landmarks.get_mut(candidate.landmark_id) {
            *used = true;
        }
        let detection = detections[candidate.detection_index].feature.pixel;
        let landmark = landmarks[candidate.landmark_id].xy;
        expected.push(ExpectedAssociation {
            detection: [detection.x(), detection.y()],
            landmark: [landmark.x(), landmark.y()],
        });
    }
    expected
}

fn new_solver_matches(
    objects: &[Object<RobocupObjectLabel>],
    camera_matrix: &CameraMatrix,
    expected: &[ExpectedAssociation],
) -> bool {
    let features = find_detected_visual_features(objects);
    let localization = localize_global_visual_features(
        &features,
        camera_matrix,
        &FieldDimensions::SPL_2025,
        None,
        &GlobalLocalizerParameters::default(),
    );
    if localization.associations.is_empty() {
        return false;
    }
    let actual = localization
        .associations
        .into_iter()
        .map(|association| {
            AssociationKey::from_points(association.detection, association.field_point.xy())
        })
        .collect::<Vec<_>>();
    let expected = expected
        .iter()
        .map(|association| AssociationKey::from_arrays(association.detection, association.landmark))
        .collect::<Vec<_>>();
    canonical_mod_symmetry(actual) == canonical_mod_symmetry(expected)
}

#[derive(Clone)]
struct Timed<T> {
    log_time: SystemTime,
    source_time: SystemTime,
    value: T,
}

#[derive(Clone, Copy)]
struct FeatureDetection {
    label: RobocupObjectLabel,
    feature: DetectedVisualFeature,
}

#[derive(Clone, Copy)]
struct FixtureLandmark {
    id: usize,
    label: RobocupObjectLabel,
    xy: Point2<Field>,
}

struct ExpectedCandidate {
    detection_index: usize,
    landmark_id: usize,
    residual: f32,
}

#[derive(Default, Debug)]
struct ExtractionStats {
    camera_messages: usize,
    detection_messages: usize,
    debug_messages: usize,
    frames_considered: usize,
    missing_camera: usize,
    stale_camera: usize,
    missing_debug: usize,
    stale_debug: usize,
    empty_debug: usize,
    non_unique_debug: usize,
    expected_count_mismatch: usize,
    too_few_expected: usize,
    solver_mismatch: usize,
    accepted: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct AssociationKey {
    detection_x: i32,
    detection_y: i32,
    landmark_x: i32,
    landmark_y: i32,
}

impl AssociationKey {
    fn from_points(detection: Point2<Pixel>, landmark: Point2<Field>) -> Self {
        Self::from_arrays([detection.x(), detection.y()], [landmark.x(), landmark.y()])
    }

    fn from_arrays(detection: [f32; 2], landmark: [f32; 2]) -> Self {
        Self {
            detection_x: quantize(detection[0]),
            detection_y: quantize(detection[1]),
            landmark_x: quantize(landmark[0]),
            landmark_y: quantize(landmark[1]),
        }
    }

    fn symmetric(self) -> Self {
        Self {
            landmark_x: -self.landmark_x,
            landmark_y: -self.landmark_y,
            ..self
        }
    }
}

fn sorted_keys(mut keys: Vec<AssociationKey>) -> Vec<AssociationKey> {
    keys.sort_unstable();
    keys
}

fn canonical_mod_symmetry(keys: Vec<AssociationKey>) -> Vec<AssociationKey> {
    let exact = sorted_keys(keys.clone());
    let symmetric = sorted_keys(keys.into_iter().map(AssociationKey::symmetric).collect());
    exact.min(symmetric)
}

fn quantize(value: f32) -> i32 {
    (value * 1000.0).round() as i32
}

fn feature_detections(objects: &[Object<RobocupObjectLabel>]) -> Vec<FeatureDetection> {
    let features = find_detected_visual_features(objects);
    let mut detections = Vec::new();
    detections.extend(
        features
            .goalposts
            .into_iter()
            .map(|feature| FeatureDetection {
                label: RobocupObjectLabel::GoalPost,
                feature,
            }),
    );
    detections.extend(
        features
            .l_spots
            .into_iter()
            .map(|feature| FeatureDetection {
                label: RobocupObjectLabel::LSpot,
                feature,
            }),
    );
    detections.extend(
        features
            .t_spots
            .into_iter()
            .map(|feature| FeatureDetection {
                label: RobocupObjectLabel::TSpot,
                feature,
            }),
    );
    detections.extend(
        features
            .penalty_spots
            .into_iter()
            .map(|feature| FeatureDetection {
                label: RobocupObjectLabel::PenaltySpot,
                feature,
            }),
    );
    detections
}

fn field_landmarks(field: &FieldDimensions) -> Vec<FixtureLandmark> {
    let mut landmarks = Vec::new();
    for point in [Half::Opponent, Half::Own]
        .into_iter()
        .flat_map(|half| [Side::Left, Side::Right].map(move |side| field.goal_post(half, side)))
    {
        push_landmark(&mut landmarks, RobocupObjectLabel::GoalPost, point);
    }
    for point in [Half::Opponent, Half::Own].into_iter().flat_map(|half| {
        [Side::Left, Side::Right].into_iter().flat_map(move |side| {
            [
                field.corner(half, side),
                field.goal_box_corner(half, side),
                field.penalty_box_corner(half, side),
            ]
        })
    }) {
        push_landmark(&mut landmarks, RobocupObjectLabel::LSpot, point);
    }
    for point in [Side::Left, Side::Right]
        .into_iter()
        .map(|side| field.t_crossing(side))
        .chain([Half::Opponent, Half::Own].into_iter().flat_map(|half| {
            [Side::Left, Side::Right].into_iter().flat_map(move |side| {
                [
                    field.goal_box_goal_line_intersection(half, side),
                    field.penalty_box_goal_line_intersection(half, side),
                ]
            })
        }))
    {
        push_landmark(&mut landmarks, RobocupObjectLabel::TSpot, point);
    }
    for point in [Half::Opponent, Half::Own]
        .into_iter()
        .map(|half| field.penalty_spot(half))
    {
        push_landmark(&mut landmarks, RobocupObjectLabel::PenaltySpot, point);
    }
    landmarks
}

fn push_landmark(
    landmarks: &mut Vec<FixtureLandmark>,
    label: RobocupObjectLabel,
    xy: Point2<Field>,
) {
    landmarks.push(FixtureLandmark {
        id: landmarks.len(),
        label,
        xy,
    });
}

fn project_landmark(
    field_to_camera: Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    landmark: Point2<Field>,
) -> Option<Point2<Pixel>> {
    let camera_point = field_to_camera * landmark.extend(0.0);
    (camera_point.z().is_finite() && camera_point.z() > 1.0e-4)
        .then(|| intrinsics.project(camera_point.coords()))
}

fn nearest_by_time<T>(items: &[Timed<T>], time: SystemTime) -> Option<&Timed<T>> {
    items
        .iter()
        .min_by_key(|item| nanos_abs_diff(item.source_time, time))
}

fn nearest_log_time<T>(items: &[Timed<T>], time: SystemTime) -> Option<&Timed<T>> {
    items
        .iter()
        .min_by_key(|item| nanos_abs_diff(item.log_time, time))
}

fn abs_duration(left: SystemTime, right: SystemTime) -> Duration {
    Duration::from_nanos(nanos_abs_diff(left, right).min(u64::MAX as u128) as u64)
}

fn nanos_abs_diff(left: SystemTime, right: SystemTime) -> u128 {
    nanos_since_epoch(left).abs_diff(nanos_since_epoch(right))
}

fn nanos_since_epoch(time: SystemTime) -> u128 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}

fn system_time_from_nanos(nanos: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_nanos(nanos)
}

fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/association_fixtures.json")
}
