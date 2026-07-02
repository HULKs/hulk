use std::f32::consts::{FRAC_PI_2, PI};

use ::types::field_dimensions::{FieldDimensions, Half, Side};
use linear_algebra::{IntoTransform, point};

use super::super::super::GlobalAssociator;
use super::super::prelude::*;
use super::assertions::assert_feature_associations_have_truth;

pub(in super::super) fn selected_goal_side_features(
    map: &LandmarkMap,
    field: &FieldDimensions,
    pose: Isometry3<Robot, Field>,
) -> Result<Vec<ProjectedSyntheticFeature>, String> {
    let landmark_ids = [
        landmark_id_at(
            map,
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Right),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.goal_box_corner(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.goal_box_corner(Half::Opponent, Side::Right),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.penalty_box_corner(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::PenaltySpot,
            field.penalty_spot(Half::Opponent),
        )?,
    ];
    let selected = projected_landmarks(map, pose)
        .into_iter()
        .filter(|feature| landmark_ids.contains(&feature.landmark_id))
        .collect::<Vec<_>>();
    if selected.len() != landmark_ids.len() {
        return Err(format!(
            "expected all targeted goal-side landmarks to be visible, got {} of {}",
            selected.len(),
            landmark_ids.len()
        ));
    }
    Ok(selected)
}

pub(in super::super) fn landmark_id(
    map: &LandmarkMap,
    class: VisualFeatureClass,
    x: f32,
    y: f32,
) -> Result<usize, String> {
    map.landmarks
        .iter()
        .find(|landmark| {
            landmark.class == class
                && (landmark.xy.x() - x).abs() < 1.0e-4
                && (landmark.xy.y() - y).abs() < 1.0e-4
        })
        .map(|landmark| landmark.id)
        .ok_or_else(|| format!("missing landmark {class:?} at ({x}, {y})"))
}

fn landmark_id_at(
    map: &LandmarkMap,
    class: VisualFeatureClass,
    point: Point2<Field>,
) -> Result<usize, String> {
    landmark_id(map, class, point.x(), point.y())
}

#[derive(Clone, Copy, Debug)]
pub(in super::super) struct ProjectedSyntheticFeature {
    landmark_id: usize,
    class: VisualFeatureClass,
    pixel: Point2<Pixel>,
}

#[derive(Debug)]
pub(in super::super) struct SyntheticFrame {
    pub(in super::super) features: DetectedVisualFeatures,
    pub(in super::super) truth_by_detection_id: Vec<usize>,
}

#[derive(Debug, Default)]
pub(in super::super) struct SyntheticStats {
    pub(in super::super) generated: usize,
    pub(in super::super) solved: usize,
    pub(in super::super) unique: usize,
    pub(in super::super) no_result: usize,
}

pub(in super::super) struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub(in super::super) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / (u32::MAX as f32 + 1.0)
    }

    fn f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }

    fn usize_below(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        self.next_u32() as usize % upper
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            let swap_with = self.usize_below(index + 1);
            items.swap(index, swap_with);
        }
    }
}

pub(in super::super) fn run_projected_subsample_cases(
    target_cases: usize,
    seed: u64,
    noise_px: f32,
) -> Result<SyntheticStats, String> {
    let field = FieldDimensions::SPL_2025;
    let config = GlobalAssociationConfig::default();
    let localizer = GlobalAssociator::new(config);
    let map = LandmarkMap::new(&field, config.min_map_baseline);
    let mut rng = DeterministicRng::new(seed);
    let mut poses = deterministic_poses(&field);
    let mut stats = SyntheticStats::default();
    let mut attempts = 0;

    while stats.generated < target_cases && attempts < target_cases * 30 {
        attempts += 1;
        let pose = poses
            .pop()
            .unwrap_or_else(|| random_pose_in_and_around_field(&field, &mut rng));
        let visible = projected_landmarks(&map, pose);
        let Some(selected) =
            random_non_collinear_subsample(&visible, config.min_inliers, &map, &mut rng)
        else {
            continue;
        };
        let frame = synthetic_frame(&add_pixel_noise(selected, &mut rng, noise_px));

        stats.generated += 1;
        let result = localizer.localize(synthetic_input(&frame.features, &field, Some(pose)));
        match result {
            Some(GlobalLocalizationResult::UniqueModuloSymmetry(result)) => {
                stats.solved += 1;
                stats.unique += 1;
                assert_feature_associations_have_truth(
                    &result.features,
                    &frame,
                    &map,
                    stats.generated,
                    "unique",
                )?;
            }
            None => {
                stats.no_result += 1;
            }
        }
    }

    Ok(stats)
}

fn deterministic_poses(field: &FieldDimensions) -> Vec<Isometry3<Robot, Field>> {
    let half_length = field.length * 0.5;
    let half_width = field.width * 0.5;
    let positions = [
        (0.0, 0.0),
        (0.6, -0.4),
        (-0.8, 0.5),
        (half_length - 1.0, 0.0),
        (-half_length + 1.0, 0.0),
        (half_length - 1.3, half_width - 0.7),
        (half_length - 1.3, -half_width + 0.7),
        (-half_length + 1.3, half_width - 0.7),
        (-half_length + 1.3, -half_width + 0.7),
        (0.0, half_width - 0.4),
        (0.0, -half_width + 0.4),
        (half_length + 0.35, 0.0),
        (-half_length - 0.35, 0.0),
        (0.0, half_width + 0.35),
        (0.0, -half_width - 0.35),
    ];
    let yaws = [0.0, FRAC_PI_2, -FRAC_PI_2, PI, 0.35 * PI, -0.65 * PI];
    let mut poses = positions
        .into_iter()
        .flat_map(|(x, y)| yaws.into_iter().map(move |yaw| robot_to_field(x, y, yaw)))
        .collect::<Vec<_>>();
    poses.reverse();
    poses
}

pub(in super::super) fn boundary_problem(field: &FieldDimensions) -> Problem {
    let cfg = GlobalAssociationConfig::default();
    Problem {
        cfg,
        k: synthetic_camera_intrinsic(),
        ground_to_robot: Isometry3::<Ground, Robot>::identity(),
        robot_to_camera: synthetic_robot_to_camera(),
        pose_hint: None,
        map: std::sync::Arc::new(LandmarkMap::new(field, cfg.min_map_baseline)),
        detections: Vec::new(),
        detections_truncated: false,
        camera_xy_ground: Vector2::zeros(),
        field_boundary: field_boundary_limits(field).expect("field boundary should be valid"),
        high_confidence_unmatched_penalty: 0.0,
    }
}

fn random_pose_in_and_around_field(
    field: &FieldDimensions,
    rng: &mut DeterministicRng,
) -> Isometry3<Robot, Field> {
    let x = rng.f32_range(-field.length * 0.5 - 0.8, field.length * 0.5 + 0.8);
    let y = rng.f32_range(-field.width * 0.5 - 0.8, field.width * 0.5 + 0.8);
    let yaw = rng.f32_range(-PI, PI);
    robot_to_field(x, y, yaw)
}

pub(in super::super) fn robot_to_field(x: f32, y: f32, yaw: f32) -> Isometry3<Robot, Field> {
    Isometry3::<Robot, Field>::wrap(nalgebra::Isometry3::from_parts(
        Translation3::new(x, y, 0.0),
        nalgebra::UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), yaw),
    ))
}

pub(in super::super) fn synthetic_input<'a>(
    features: &'a DetectedVisualFeatures,
    field_dimensions: &'a FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
) -> GlobalLocalizationInput<'a> {
    GlobalLocalizationInput {
        visual_features: features,
        field_dimensions,
        ground_to_robot: Isometry3::<Ground, Robot>::identity(),
        robot_to_camera: synthetic_robot_to_camera(),
        camera_intrinsic: synthetic_camera_intrinsic(),
        pose_hint,
    }
}

fn synthetic_robot_to_camera() -> Isometry3<Robot, Camera> {
    nalgebra::Isometry3::translation(0.0, 0.0, 0.5).framed_transform()
}

fn synthetic_camera_intrinsic() -> Intrinsic {
    Intrinsic::new(nalgebra::vector![60.0, 60.0], point![<Pixel>, 640.0, 480.0])
}

pub(in super::super) fn projected_landmarks(
    map: &LandmarkMap,
    robot_to_field: Isometry3<Robot, Field>,
) -> Vec<ProjectedSyntheticFeature> {
    let field_to_camera = synthetic_robot_to_camera() * robot_to_field.inverse();
    let intrinsic = synthetic_camera_intrinsic();
    map.landmarks
        .iter()
        .filter_map(|landmark| {
            let camera_point = field_to_camera * landmark.xy.extend(0.0);
            if !camera_point.z().is_finite() || camera_point.z() <= 1.0e-4 {
                return None;
            }
            let pixel = intrinsic.project(camera_point.coords());
            pixel_inside_image(pixel, 4.0).then_some(ProjectedSyntheticFeature {
                landmark_id: landmark.id,
                class: landmark.class,
                pixel,
            })
        })
        .collect()
}

fn pixel_inside_image(pixel: Point2<Pixel>, margin: f32) -> bool {
    pixel.x().is_finite()
        && pixel.y().is_finite()
        && pixel.x() >= margin
        && pixel.x() <= 1280.0 - margin
        && pixel.y() >= margin
        && pixel.y() <= 960.0 - margin
}

pub(in super::super) fn random_non_collinear_subsample(
    visible: &[ProjectedSyntheticFeature],
    min_count: usize,
    map: &LandmarkMap,
    rng: &mut DeterministicRng,
) -> Option<Vec<ProjectedSyntheticFeature>> {
    if visible.len() < min_count.max(3) {
        return None;
    }
    let max_count = visible.len().min(8);
    let min_count = min_count.max(3).min(max_count);
    for _ in 0..64 {
        let mut selected = visible.to_vec();
        rng.shuffle(&mut selected);
        selected.truncate(min_count + rng.usize_below(max_count - min_count + 1));
        if has_useful_class_mix(&selected, visible)
            && has_non_collinear_landmark_triplet(&selected, map)
        {
            return Some(selected);
        }
    }
    None
}

fn has_useful_class_mix(
    selected: &[ProjectedSyntheticFeature],
    visible: &[ProjectedSyntheticFeature],
) -> bool {
    class_count(selected) >= class_count(visible).min(2)
}

fn class_count(features: &[ProjectedSyntheticFeature]) -> usize {
    FEATURE_CLASSES
        .into_iter()
        .filter(|class| features.iter().any(|feature| feature.class == *class))
        .count()
}

fn has_non_collinear_landmark_triplet(
    selected: &[ProjectedSyntheticFeature],
    map: &LandmarkMap,
) -> bool {
    for first in 0..selected.len() {
        for second in 0..selected.len() {
            if second == first {
                continue;
            }
            for third in 0..selected.len() {
                if third == first || third == second {
                    continue;
                }
                let a = map.landmarks[selected[first].landmark_id].xy;
                let b = map.landmarks[selected[second].landmark_id].xy;
                let c = map.landmarks[selected[third].landmark_id].xy;
                let v = (b - a).inner;
                let w = (c - a).inner;
                let norm_squared = v.norm_squared();
                if norm_squared <= 0.25 {
                    continue;
                }
                let beta = (v.x * w.y - v.y * w.x) / norm_squared;
                if beta.abs() >= 0.05 {
                    return true;
                }
            }
        }
    }
    false
}

pub(in super::super) fn add_pixel_noise(
    mut selected: Vec<ProjectedSyntheticFeature>,
    rng: &mut DeterministicRng,
    noise_px: f32,
) -> Vec<ProjectedSyntheticFeature> {
    for feature in &mut selected {
        feature.pixel = point![<Pixel>,
            feature.pixel.x() + rng.f32_range(-noise_px, noise_px),
            feature.pixel.y() + rng.f32_range(-noise_px, noise_px)
        ];
    }
    selected
}

pub(in super::super) fn synthetic_frame(selected: &[ProjectedSyntheticFeature]) -> SyntheticFrame {
    let mut frame = SyntheticFrame {
        features: DetectedVisualFeatures::default(),
        truth_by_detection_id: Vec::new(),
    };
    for class in [
        VisualFeatureClass::GoalPost,
        VisualFeatureClass::LSpot,
        VisualFeatureClass::TSpot,
        VisualFeatureClass::XSpot,
        VisualFeatureClass::PenaltySpot,
    ] {
        for feature in selected.iter().filter(|feature| feature.class == class) {
            push_synthetic_feature(&mut frame.features, class, feature.pixel);
            frame.truth_by_detection_id.push(feature.landmark_id);
        }
    }
    frame
}

fn push_synthetic_feature(
    features: &mut DetectedVisualFeatures,
    class: VisualFeatureClass,
    pixel: Point2<Pixel>,
) {
    let detection = DetectedVisualFeature {
        pixel,
        confidence: 0.95,
    };
    match class {
        VisualFeatureClass::GoalPost => features.goalposts.push(detection),
        VisualFeatureClass::LSpot => features.l_spots.push(detection),
        VisualFeatureClass::TSpot => features.t_spots.push(detection),
        VisualFeatureClass::XSpot => features.x_spots.push(detection),
        VisualFeatureClass::PenaltySpot => features.penalty_spots.push(detection),
    }
}
