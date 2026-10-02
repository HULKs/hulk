use coordinate_systems::{Camera, Field, Local, Robot};
use linear_algebra::{Isometry2, Isometry3, Point2, point};
use nalgebra::{SMatrix, UnitQuaternion, vector};
use projection::intrinsic::Intrinsic;
use ros_z::time::Time;
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    localization::PoseEstimate,
    visual_localization::AssociationGeometry,
};

use super::{GlobalAssociationConfig, solver};
use crate::{
    AssociationResult, DetectedVisualFeature, DetectedVisualFeatures,
    FieldMarkAssociationParameters, GlobalAssociationInput,
    TrackingAssociationInput as AssociationInput, VisualFeatureClass,
    associate_global_visual_features, associate_tracking_visual_features, map::LandmarkMap,
};

fn geometry() -> AssociationGeometry {
    AssociationGeometry {
        epoch: 0,
        generation: 0,
        estimate: tracking_estimate(0.0),
        last_successful_solve: Time::from_nanos(1_000_000_000),
    }
}

fn intrinsic() -> Intrinsic {
    Intrinsic::new(vector![500.0, 500.0], point![320.0, 240.0])
}

fn robot_to_camera() -> Isometry3<Robot, Camera> {
    Isometry3::wrap(
        nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 0.0, 0.1),
            UnitQuaternion::from_euler_angles(std::f32::consts::PI, 0.0, 0.0),
        )
        .inverse(),
    )
}

fn input<'a>(
    features: &'a DetectedVisualFeatures,
    geometry: &'a AssociationGeometry,
) -> AssociationInput<'a> {
    AssociationInput {
        visual_features: features,
        robot_to_camera: robot_to_camera(),
        geometry,
        camera_intrinsic: intrinsic(),
        field_dimensions: &FieldDimensions::SPL_2025,
        time: Time::from_nanos(1_000_000_000),
    }
}

fn global_input<'a>(
    input: AssociationInput<'a>,
    parameters: &'a GlobalAssociationConfig,
) -> GlobalAssociationInput<'a> {
    let (roll, pitch, _) = input.geometry.estimate.pose.inner.rotation.euler_angles();
    GlobalAssociationInput {
        visual_features: input.visual_features,
        robot_to_ground: linear_algebra::Rotation3::from_euler_angles(
            roll as f32,
            pitch as f32,
            0.0,
        ),
        robot_to_camera: input.robot_to_camera,
        camera_intrinsic: input.camera_intrinsic,
        field_dimensions: input.field_dimensions,
        parameters,
        heading: None,
    }
}

fn calibrated_parameters() -> FieldMarkAssociationParameters {
    let mut parameters = FieldMarkAssociationParameters::default();
    parameters.global_localizer.imu_tilt_sigma = 0.001;
    parameters.global_localizer.height_sigma = 0.001;
    parameters
}

fn project(
    landmarks: impl IntoIterator<Item = (VisualFeatureClass, Point2<Field>)>,
    geometry: &AssociationGeometry,
) -> DetectedVisualFeatures {
    let field_to_camera = robot_to_camera()
        * Isometry3::<Robot, Field>::wrap(geometry.estimate.pose.inner.cast()).inverse();
    let mut features = DetectedVisualFeatures::default();
    for (class, landmark) in landmarks {
        let pixel = intrinsic().project((field_to_camera * landmark.extend(0.0)).coords());
        let feature = DetectedVisualFeature {
            pixel,
            confidence: 0.95,
        };
        match class {
            VisualFeatureClass::GoalPost => &mut features.goalposts,
            VisualFeatureClass::LSpot => &mut features.l_spots,
            VisualFeatureClass::TSpot => &mut features.t_spots,
            VisualFeatureClass::XSpot => &mut features.x_spots,
            VisualFeatureClass::PenaltySpot => &mut features.penalty_spots,
        }
        .push(feature);
    }
    features
}

fn stationary_three() -> [(VisualFeatureClass, Point2<Field>); 3] {
    let field = FieldDimensions::SPL_2025;
    [
        (
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Left),
        ),
        (
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Right),
        ),
        (
            VisualFeatureClass::PenaltySpot,
            field.penalty_spot(Half::Opponent),
        ),
    ]
}

fn key(result: &AssociationResult) -> Vec<([f32; 2], [f32; 2])> {
    result
        .associations
        .iter()
        .map(|a| {
            (
                [a.detection.x(), a.detection.y()],
                [a.field_point.x(), a.field_point.y()],
            )
        })
        .collect()
}

fn tracking_estimate(covariance: f64) -> PoseEstimate<Robot, Field> {
    PoseEstimate {
        pose: Isometry3::wrap(nalgebra::Isometry3::translation(2.0, 0.0, 0.45)),
        covariance: SMatrix::identity() * covariance,
    }
}

#[test]
fn geometric_numerical_guards_are_applied() {
    let geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let baseline = GlobalAssociationConfig::default();
    let associate = |config: &GlobalAssociationConfig| {
        associate_global_visual_features(global_input(input(&features, &geometry), config))
    };
    assert_eq!(associate(&baseline).associations.len(), 3);
    for config in [
        GlobalAssociationConfig {
            min_pair_distance: 1.0e6,
            ..baseline
        },
        GlobalAssociationConfig {
            min_triangle_denominator: 1.0e12,
            ..baseline
        },
    ] {
        config.validate().unwrap();
        assert!(associate(&config).associations.is_empty());
    }
}

#[test]
fn three_stationary_features_certify_without_history_or_pose() {
    let geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let mut parameters = FieldMarkAssociationParameters::default();
    assert_eq!(parameters.global_localizer.min_inliers, 3);
    let first = associate_global_visual_features(global_input(
        input(&features, &geometry),
        &parameters.global_localizer,
    ));
    assert_eq!(first.associations.len(), 3);
    assert!(first.associations.iter().all(|a| a.field_point.x() < 0.0));
    assert_eq!(first.debug.as_ref().unwrap().association_count, 3);
    assert!(first.debug.as_ref().unwrap().pairwise_distance_rms < 1.0e-4);
    // Global fitting estimates height; tracking's height-noise floor is irrelevant.
    parameters.global_localizer.height_sigma = 0.2;
    assert_eq!(
        key(&first),
        key(&associate_global_visual_features(global_input(
            input(&features, &geometry),
            &parameters.global_localizer
        )))
    );
}

#[test]
fn lifecycle_dispatch_uses_exposure_attitude_and_matches_generations() {
    use types::localization::{FieldHeadingReference, LocalizationState, LocalizationStatus};
    let geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let parameters = FieldMarkAssociationParameters::default();
    let mut wrong = geometry.clone();
    wrong.estimate = tracking_estimate(0.01);
    wrong.estimate.pose.inner.rotation = UnitQuaternion::from_euler_angles(0.8, 0.7, 1.2);
    let mut status = LocalizationStatus {
        time: Time::from_nanos(0),
        epoch: 0,
        generation: 0,
        state: LocalizationState::Startup,
        heading: None,
    };
    let call = |status: &LocalizationStatus, tracking| {
        crate::associate_visual_features(
            crate::AssociationInput {
                status,
                attitude: Some(linear_algebra::Orientation3::from_euler_angles(
                    0.0, 0.0, 1.3,
                )),
                tracking,
                visual_features: &features,
                robot_to_camera: robot_to_camera(),
                camera_intrinsic: intrinsic(),
                field_dimensions: &FieldDimensions::SPL_2025,
                time: Time::from_nanos(1_000_000_000),
            },
            &parameters,
        )
    };
    // Startup must ignore an optimizer tilt that disagrees with the source IMU.
    assert_eq!(call(&status, Some(&wrong)).associations.len(), 3);
    status.state = LocalizationState::LostTrack;
    assert!(call(&status, None).associations.is_empty());
    status.heading = Some(FieldHeadingReference {
        time: status.time,
        imu_to_field: linear_algebra::Rotation2::new(-1.3),
        max_error: 0.3,
    });
    let recovery = call(&status, None);
    assert_eq!(
        recovery.source,
        types::visual_localization::VisualAssociationSource::Global
    );
    assert_eq!(recovery.associations.len(), 3);
    assert!(
        recovery
            .associations
            .iter()
            .all(|a| a.field_point.x() > 0.0)
    );
    status.state = LocalizationState::Tracking;
    status.generation = 1;
    assert!(call(&status, Some(&wrong)).associations.is_empty());
}

#[test]
fn heading_preserves_orientation_and_budget_exhaustion_still_rejects() {
    let geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let config = GlobalAssociationConfig::default();
    let mut input = global_input(input(&features, &geometry), &config);
    let heading = crate::HeadingConstraint {
        expected: linear_algebra::Orientation2::identity(),
        max_error: 0.3,
    };
    input.heading = Some(heading);
    let result = associate_global_visual_features(input);
    assert_eq!(result.associations.len(), 3);
    assert!(result.associations.iter().all(|a| a.field_point.x() > 0.0));
    let exhausted = GlobalAssociationConfig {
        max_work: 1,
        ..config
    };
    assert!(
        associate_global_visual_features(GlobalAssociationInput {
            parameters: &exhausted,
            ..input
        })
        .associations
        .is_empty()
    );
    for limit in [f64::NAN, 0.0, std::f64::consts::FRAC_PI_2] {
        input.heading = Some(crate::HeadingConstraint {
            max_error: limit,
            ..heading
        });
        assert!(
            associate_global_visual_features(input)
                .associations
                .is_empty()
        );
    }
}

#[test]
fn canonical_landmarks_do_not_select_the_robot_half() {
    for robot_x in [-2.0, 2.0] {
        let mut geometry = geometry();
        geometry.estimate.pose.inner.translation.vector.x = robot_x;
        let features = project(stationary_three(), &geometry);
        let config = calibrated_parameters().global_localizer;
        let result =
            associate_global_visual_features(global_input(input(&features, &geometry), &config));
        assert_eq!(result.associations.len(), 3, "robot x {robot_x}");
        assert!(result.associations.iter().all(|a| a.field_point.x() < 0.0));
        // These correspondences imply a half-turn, so the canonical robot is at -robot_x.
        // In particular, a robot really at -2 yields a representative at +2, not own-half.
        for (association, (_, expected)) in result.associations.iter().zip(stationary_three()) {
            assert!(
                (association.field_point.xy().coords().inner + expected.coords().inner).norm()
                    < 1.0e-5
            );
        }
    }
}

#[test]
fn halfturn_equivalent_triangles_are_not_ambiguous() {
    let geometry = geometry();
    let field = FieldDimensions::SPL_2025;
    // Swapping the two penalties and the circle crossing is exactly a field half-turn.
    let features = project(
        [
            (
                VisualFeatureClass::PenaltySpot,
                field.penalty_spot(Half::Own),
            ),
            (
                VisualFeatureClass::PenaltySpot,
                field.penalty_spot(Half::Opponent),
            ),
            (VisualFeatureClass::XSpot, field.x_crossing(Side::Left)),
        ],
        &geometry,
    );
    let config = calibrated_parameters().global_localizer;
    let result =
        associate_global_visual_features(global_input(input(&features, &geometry), &config));
    assert_eq!(result.associations.len(), 3);
}

#[test]
fn translated_triangle_orbits_are_rejected() {
    let geometry = geometry();
    let field = FieldDimensions {
        penalty_area_length: FieldDimensions::SPL_2025.goal_box_area_length,
        ..FieldDimensions::SPL_2025
    };
    // Translation by -(goal_box_width + penalty_box_width)/2 yields another L,T,T
    // triangle on the same goal line, with identical edge lengths AND chirality.
    let features = project(
        [
            (
                VisualFeatureClass::LSpot,
                field.goal_box_corner(Half::Opponent, Side::Left),
            ),
            (
                VisualFeatureClass::TSpot,
                field.goal_box_goal_line_intersection(Half::Opponent, Side::Left),
            ),
            (
                VisualFeatureClass::TSpot,
                field.penalty_box_goal_line_intersection(Half::Opponent, Side::Left),
            ),
        ],
        &geometry,
    );
    let config = calibrated_parameters().global_localizer;
    let mut input = input(&features, &geometry);
    input.field_dimensions = &field;
    assert!(
        associate_global_visual_features(global_input(input, &config))
            .associations
            .is_empty()
    );
}

#[test]
fn collinear_and_duplicate_seeds_do_not_certify() {
    let geometry = geometry();
    let field = FieldDimensions::SPL_2025;
    let features = project(
        [
            (VisualFeatureClass::XSpot, field.center()),
            (VisualFeatureClass::XSpot, field.x_crossing(Side::Left)),
            (VisualFeatureClass::XSpot, field.x_crossing(Side::Right)),
        ],
        &geometry,
    );
    assert!(
        associate_global_visual_features(global_input(
            input(&features, &geometry),
            &GlobalAssociationConfig::default()
        ))
        .associations
        .is_empty()
    );
    let mut features = project(stationary_three(), &geometry);
    features.goalposts[1] = features.goalposts[0];
    assert!(
        associate_global_visual_features(global_input(
            input(&features, &geometry),
            &GlobalAssociationConfig::default()
        ))
        .associations
        .is_empty()
    );
}

#[test]
fn rich_frame_has_bounded_search_and_budget_exhaustion_rejects() {
    let geometry = geometry();
    let map = LandmarkMap::new(
        &FieldDimensions::SPL_2025,
        GlobalAssociationConfig::default().symmetry_epsilon,
    );
    let features = project(map.landmarks.iter().map(|l| (l.class, l.xy)), &geometry);
    let mut config = calibrated_parameters().global_localizer;
    let result =
        associate_global_visual_features(global_input(input(&features, &geometry), &config));
    assert_eq!(result.associations.len(), map.landmarks.len());
    config.max_work = 1;
    assert!(
        associate_global_visual_features(global_input(input(&features, &geometry), &config))
            .associations
            .is_empty()
    );
}

#[test]
fn tracking_keeps_opponent_half_prediction() {
    let mut geometry = geometry();
    geometry.estimate = tracking_estimate(0.0001);
    let features = project(stationary_three(), &geometry);
    let result = associate_tracking_visual_features(
        input(&features, &geometry),
        &FieldMarkAssociationParameters::default(),
    );
    assert_eq!(result.associations.len(), 3);
    assert!(result.associations.iter().all(|a| a.field_point.x() > 0.0));
    geometry.estimate.pose.inner.translation.x = f64::NAN;
    assert!(
        associate_tracking_visual_features(
            input(&features, &geometry),
            &FieldMarkAssociationParameters::default()
        )
        .associations
        .is_empty()
    );
}

#[test]
fn tracking_covariance_and_age_widen_the_gate_but_not_the_distance_limit() {
    let mut geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let mut parameters = calibrated_parameters();
    // Generate observations from truth before shifting the single prior by 20 pixels.
    geometry.estimate.pose.inner.translation.x += 20.0 * 0.55 / 500.0;
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
    let mut older = input(&features, &geometry);
    older.time = Time::from_nanos(2_000_000_000);
    assert_eq!(
        associate_tracking_visual_features(older, &parameters)
            .associations
            .len(),
        3
    );
    geometry.estimate.covariance = SMatrix::identity() * 0.001;
    assert_eq!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .len(),
        3
    );
    parameters.tracking.max_pixel_distance = 19.0;
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
}

#[test]
fn invalid_geometry_and_input_caps_fail_closed() {
    let mut geometry = geometry();
    let mut features = project(stationary_three(), &geometry);
    let config = GlobalAssociationConfig::default();
    let mut invalid = input(&features, &geometry);
    invalid.camera_intrinsic.focals.x = f32::NAN;
    assert!(
        associate_global_visual_features(global_input(invalid, &config))
            .associations
            .is_empty()
    );
    let mut over_cap = project(
        (0..33).map(|i| {
            (
                VisualFeatureClass::LSpot,
                point![2.0 + i as f32 * 0.01, 0.0],
            )
        }),
        &geometry,
    );
    assert!(solver::preprocess(global_input(input(&over_cap, &geometry), &config)).is_none());
    over_cap.l_spots[32] = over_cap.l_spots[0];
    assert_eq!(
        solver::preprocess(global_input(input(&over_cap, &geometry), &config))
            .unwrap()
            .1
            .len(),
        32
    );
    geometry.estimate.pose.inner.rotation = UnitQuaternion::from_euler_angles(f64::NAN, 0.0, 0.0);
    assert!(
        associate_global_visual_features(global_input(input(&features, &geometry), &config))
            .associations
            .is_empty()
    );
    geometry.estimate.pose.inner.rotation = UnitQuaternion::identity();
    features.goalposts = vec![features.goalposts[0]; 129];
    assert!(
        associate_global_visual_features(global_input(input(&features, &geometry), &config))
            .associations
            .is_empty()
    );
}

#[test]
fn global_is_invariant_under_local_translation_yaw_and_detection_order() {
    let config = calibrated_parameters().global_localizer;
    for yaw in [-2.4, 0.0, 1.7] {
        let local_to_field =
            Isometry2::<Local, Field>::wrap(nalgebra::Isometry2::new(vector![0.7, -0.3], yaw));
        let mut geometry = geometry();
        let local_robot = local_to_field.inverse() * point![2.0, 0.0];
        let robot_to_local = Isometry3::<Robot, Local>::wrap(nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(local_robot.x(), local_robot.y(), 0.45),
            UnitQuaternion::from_euler_angles(0.0, 0.0, -yaw),
        ));
        // Local coordinates are only a test construction of the same field pose.
        geometry.estimate.pose =
            Isometry3::wrap((local_to_field.to_3d() * robot_to_local).inner.cast());
        let mut features = project(stationary_three(), &geometry);
        for _ in 0..2 {
            let result = associate_global_visual_features(global_input(
                input(&features, &geometry),
                &config,
            ));
            assert_eq!(result.associations.len(), 3);
            // Detection reordering may choose the other half-turn representative, but never a reflection.
            let (_, detections) =
                solver::preprocess(global_input(input(&features, &geometry), &config)).unwrap();
            let expected = result
                .associations
                .iter()
                .map(|association| {
                    let detection = detections
                        .iter()
                        .find(|d| d.pixel == association.detection)
                        .unwrap();
                    let robot_to_field =
                        Isometry3::<Robot, Field>::wrap(geometry.estimate.pose.inner.cast());
                    let camera_to_field = robot_to_field * robot_to_camera().inverse();
                    let origin = camera_to_field.translation().coords().inner;
                    let ground_to_field =
                        nalgebra::UnitComplex::new(robot_to_field.inner.rotation.euler_angles().2);
                    let field = origin.xy() + origin.z * (ground_to_field * detection.xy);
                    (field, association.field_point.xy().coords().inner)
                })
                .collect::<Vec<_>>();
            assert!(
                [1.0, -1.0]
                    .into_iter()
                    .any(|sign| expected.iter().all(|(a, b)| (sign * a - b).norm() < 1.0e-4))
            );
            features.goalposts.reverse();
        }
    }
}

#[test]
fn projection_covariance_matches_finite_differences_with_camera_lever_arm() {
    let mut geometry = geometry();
    geometry.estimate.pose.inner.rotation = UnitQuaternion::from_euler_angles(0.1, -0.15, 0.7);
    let camera_to_robot = Isometry3::<Camera, Robot>::wrap(nalgebra::Isometry3::from_parts(
        nalgebra::Translation3::new(0.25, -0.13, 0.15),
        UnitQuaternion::from_euler_angles(2.9, 0.1, -0.2),
    ));
    let pixel = intrinsic().project(
        (camera_to_robot.inverse()
            * Isometry3::<Robot, Field>::wrap(geometry.estimate.pose.inner.cast()).inverse()
            * point![<Field>, 3.0, -0.4, 0.0])
        .coords(),
    );
    let features = DetectedVisualFeatures {
        penalty_spots: vec![DetectedVisualFeature {
            pixel,
            confidence: 1.0,
        }],
        ..Default::default()
    };
    let config = GlobalAssociationConfig::default();
    let ground_rotation = global_input(input(&features, &geometry), &config).robot_to_ground;
    let projected = |rotation, features: &DetectedVisualFeatures| {
        let mut input = input(features, &geometry);
        input.robot_to_camera = camera_to_robot.inverse();
        let mut input = global_input(input, &config);
        input.robot_to_ground = rotation;
        solver::preprocess(input).unwrap().1.remove(0)
    };
    let actual = projected(ground_rotation, &features).covariance(config);
    let mut numerical = nalgebra::Matrix2::identity() * 1.0e-8;
    for axis in [nalgebra::Vector3::x(), nalgebra::Vector3::y()] {
        let plus = linear_algebra::Rotation3::wrap(
            UnitQuaternion::from_scaled_axis(axis * 0.001) * ground_rotation.inner,
        );
        let minus = linear_algebra::Rotation3::wrap(
            UnitQuaternion::from_scaled_axis(axis * -0.001) * ground_rotation.inner,
        );
        let derivative = (projected(plus, &features).xy - projected(minus, &features).xy) / 0.002;
        numerical += config.imu_tilt_sigma.powi(2) * derivative * derivative.transpose();
    }
    for delta in [point![0.1, 0.0], point![0.0, 0.1]] {
        let shifted = |sign| DetectedVisualFeatures {
            penalty_spots: vec![DetectedVisualFeature {
                pixel: pixel + delta.coords() * sign,
                confidence: 1.0,
            }],
            ..Default::default()
        };
        let derivative = (projected(ground_rotation, &shifted(1.0)).xy
            - projected(ground_rotation, &shifted(-1.0)).xy)
            / 0.2;
        numerical += config.detection_pixel_sigma.powi(2) * derivative * derivative.transpose();
    }
    assert!(
        (actual - numerical).norm() < numerical.norm() * 0.005,
        "actual {actual}, numerical {numerical}"
    );
}

#[test]
fn pixel_uncertainty_reaching_horizon_is_rejected() {
    let geometry = geometry();
    let features = DetectedVisualFeatures {
        penalty_spots: vec![DetectedVisualFeature {
            pixel: point![320.0, 235.0],
            confidence: 1.0,
        }],
        ..Default::default()
    };
    let mut input = input(&features, &geometry);
    input.robot_to_camera = Isometry3::wrap(nalgebra::Isometry3::rotation(vector![
        std::f32::consts::FRAC_PI_2,
        0.0,
        0.0
    ]))
    .inverse();
    let config = GlobalAssociationConfig {
        detection_pixel_sigma: 0.1,
        imu_tilt_sigma: 0.0001,
        ..Default::default()
    };
    assert_eq!(
        solver::preprocess(global_input(input, &config))
            .unwrap()
            .1
            .len(),
        1
    );
    assert!(
        solver::preprocess(global_input(
            input,
            &GlobalAssociationConfig {
                detection_pixel_sigma: 2.0,
                ..config
            }
        ))
        .unwrap()
        .1
        .is_empty()
    );
}

#[test]
fn tracking_rejects_invalid_covariance_future_solve_and_ambiguous_assignment() {
    let mut geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let parameters = FieldMarkAssociationParameters::default();
    let mut invalid_covariances = vec![SMatrix::identity() * f64::NAN, -SMatrix::identity()];
    let mut nonsymmetric = SMatrix::identity();
    nonsymmetric[(0, 1)] = 0.1;
    invalid_covariances.push(nonsymmetric);
    let mut indefinite = SMatrix::identity();
    indefinite[(0, 1)] = 2.0;
    indefinite[(1, 0)] = 2.0;
    invalid_covariances.push(indefinite);
    for covariance in invalid_covariances {
        geometry.estimate.covariance = covariance;
        assert!(
            associate_tracking_visual_features(input(&features, &geometry), &parameters)
                .associations
                .is_empty()
        );
    }
    geometry.estimate = tracking_estimate(0.0001);
    let mut future = input(&features, &geometry);
    future.time = Time::from_nanos(999_999_999);
    assert!(
        associate_tracking_visual_features(future, &parameters)
            .associations
            .is_empty()
    );
    geometry.estimate = tracking_estimate(1.0);
    let field = FieldDimensions::SPL_2025;
    let features = project(
        [
            stationary_three()[0],
            stationary_three()[1],
            (
                VisualFeatureClass::XSpot,
                point![0.0, field.center_circle_diameter / 4.0],
            ),
        ],
        &geometry,
    );
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
}

#[test]
fn global_never_certifies_a_seed_subset_with_a_competing_penalty() {
    let geometry = geometry();
    let config = calibrated_parameters().global_localizer;
    let mut features = project(
        stationary_three()
            .into_iter()
            .chain([(VisualFeatureClass::PenaltySpot, point![5.8, 0.0])]),
        &geometry,
    );
    for confidences in [[0.4, 0.95], [0.95, 0.4]] {
        for (feature, confidence) in features.penalty_spots.iter_mut().zip(confidences) {
            feature.confidence = confidence;
        }
        assert_eq!(
            solver::preprocess(global_input(input(&features, &geometry), &config))
                .unwrap()
                .1
                .len(),
            4
        );
        assert!(
            associate_global_visual_features(global_input(input(&features, &geometry), &config))
                .associations
                .is_empty()
        );
    }
    // Each competing triple alone is certifiable; retaining both must not silently omit either.
    for penalty in features.penalty_spots.clone() {
        let triple = DetectedVisualFeatures {
            goalposts: features.goalposts.clone(),
            penalty_spots: vec![penalty],
            ..Default::default()
        };
        assert_eq!(
            associate_global_visual_features(global_input(input(&triple, &geometry), &config))
                .associations
                .len(),
            3
        );
    }
}

#[test]
fn tracking_joint_assignment_keeps_plausible_rivals_outside_the_distance_limit() {
    let mut geometry = geometry();
    geometry.estimate.pose.inner.translation.vector.y = 0.375;
    let features = project(
        [
            stationary_three()[0],
            stationary_three()[1],
            stationary_three()[2],
            (VisualFeatureClass::XSpot, point![0.0, 0.365]),
        ],
        &geometry,
    );
    let mut parameters = FieldMarkAssociationParameters::default();
    parameters.tracking.max_pixel_distance = 340.0;
    // The center and +y crossing have mirror-symmetric image covariances about robot y=0.375.
    // This observation is 332 pixels from the center and 350 from its plausible rival.
    // Pruning that rival at the output ceiling would falsely certify the center match.
    geometry.estimate = tracking_estimate(1.0);
    geometry.estimate.pose.inner.translation.vector.y = 0.375;
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
}

#[test]
fn tracking_age_is_a_validity_horizon_including_its_exact_boundary() {
    let mut geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let mut parameters = calibrated_parameters();
    parameters.tracking.position_sigma_per_second = 1.0e-6;
    parameters.tracking.yaw_sigma_per_second = 1.0e-6;
    geometry.estimate = tracking_estimate(0.0);
    let mut input = input(&features, &geometry);
    input.time = input.time + parameters.tracking.max_age;
    assert_eq!(
        associate_tracking_visual_features(input, &parameters)
            .associations
            .len(),
        3
    );
    input.time = input.time + std::time::Duration::from_nanos(1);
    assert!(
        associate_tracking_visual_features(input, &parameters)
            .associations
            .is_empty()
    );
}

#[test]
fn tracking_high_covariance_keeps_distinct_opponent_landmarks_and_exact_age_horizon() {
    let mut geometry = geometry();
    let features = project(stationary_three(), &geometry);
    let expected = [
        features.goalposts[0],
        features.goalposts[1],
        features.penalty_spots[0],
    ]
    .into_iter()
    .zip(stationary_three())
    .map(|(feature, (_, point))| (feature.pixel, point))
    .collect::<Vec<_>>();
    let parameters = FieldMarkAssociationParameters::default();
    geometry.estimate.covariance = sparse_retained_covariance();
    for age in [
        std::time::Duration::from_millis(3100),
        parameters.tracking.max_age,
    ] {
        let mut input = input(&features, &geometry);
        input.time = input.time + age;
        let result = associate_tracking_visual_features(input, &parameters);
        assert_eq!(result.associations.len(), 3, "age={age:?}");
        for association in &result.associations {
            assert!(expected.contains(&(association.detection, association.field_point.xy())));
            assert!(association.field_point.x() > 0.0);
        }
    }
}

fn sparse_retained_covariance() -> nalgebra::Matrix6<f64> {
    // Retained diagonal from the real sparse simulator immediately before loss at 7.9 s.
    SMatrix::from_diagonal(&nalgebra::Vector6::new(
        0.00066995865,
        0.000579513,
        0.002903463,
        1.0588479,
        0.9564871,
        0.9908409,
    ))
}

#[test]
fn tracking_uses_raw_image_features_even_when_their_rays_reach_the_horizon() {
    let mut geometry = geometry();
    geometry.estimate.pose = Isometry3::from_translation(0.0, 0.0, 0.45);
    let robot_to_camera = Isometry3::<Robot, Camera>::wrap(nalgebra::Isometry3::rotation(vector![
        0.0,
        -std::f32::consts::FRAC_PI_2,
        0.0
    ]));
    let observe = |field: Point2<Field>| {
        let pixel = intrinsic().project(
            (robot_to_camera
                * Isometry3::<Robot, Field>::wrap(geometry.estimate.pose.inner.cast()).inverse()
                * field.extend(0.0))
            .coords(),
        );
        // These observed rays are exactly horizontal. Shared uncertain camera height explains
        // their elevation error, but intersecting them with the ground cannot produce a point.
        DetectedVisualFeature {
            pixel: point![320.0, pixel.y()],
            confidence: 0.95,
        }
    };
    let mut features = DetectedVisualFeatures {
        goalposts: vec![
            observe(stationary_three()[0].1),
            observe(stationary_three()[1].1),
        ],
        penalty_spots: vec![observe(stationary_three()[2].1)],
        ..Default::default()
    };
    features.goalposts.push(DetectedVisualFeature {
        pixel: point![f32::NAN, 0.0],
        confidence: 1.0,
    });
    features.penalty_spots.push(DetectedVisualFeature {
        pixel: point![10.0, 10.0],
        confidence: f32::NAN,
    });
    let mut estimate = PoseEstimate {
        pose: geometry.estimate.pose,
        covariance: SMatrix::zeros(),
    };
    estimate.covariance[(5, 5)] = 0.25_f64.powi(2);
    geometry.estimate = estimate;
    let mut input = input(&features, &geometry);
    input.robot_to_camera = robot_to_camera;
    assert!(
        solver::preprocess(global_input(input, &GlobalAssociationConfig::default()))
            .unwrap()
            .1
            .is_empty()
    );
    let result =
        associate_tracking_visual_features(input, &FieldMarkAssociationParameters::default());
    assert_eq!(result.associations.len(), 3);
    assert!(result.associations.iter().all(|a| a.field_point.x() > 0.0));
    assert!(
        result.debug.is_none(),
        "pixel residuals must not be labeled as metric RMS"
    );
}

#[test]
fn tracking_process_noise_widens_the_gate_at_single_prior_ranges() {
    let parameters = calibrated_parameters();
    for robot_x in [1.0, 2.0] {
        let mut geometry = geometry();
        geometry.estimate.pose.inner.translation.x = robot_x;
        let features = project(stationary_three(), &geometry);
        // Keep stored covariance zero; age-dependent noise alone must explain the offset.
        geometry.estimate.pose.inner.translation.x += 20.0 * 0.55 / 500.0;
        let mut input = input(&features, &geometry);
        assert!(
            associate_tracking_visual_features(input, &parameters)
                .associations
                .is_empty()
        );
        input.time = input.time + std::time::Duration::from_secs(1);
        assert_eq!(
            associate_tracking_visual_features(input, &parameters)
                .associations
                .len(),
            3,
            "robot x {robot_x}"
        );
    }
}

#[test]
fn tracking_filters_duplicates_but_rejects_input_and_retained_overflow() {
    let mut geometry = geometry();
    geometry.estimate = tracking_estimate(0.0001);
    let parameters = FieldMarkAssociationParameters::default();
    let mut features = project(stationary_three(), &geometry);
    let duplicate = features.goalposts[0];
    features.goalposts.push(duplicate);
    assert_eq!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .len(),
        3
    );
    features
        .goalposts
        .extend(std::iter::repeat_n(duplicate, 125));
    assert_eq!(features.supported_feature_count(), 129);
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
    let mut features = project(stationary_three(), &geometry);
    features
        .goalposts
        .extend((0..30).map(|i| DetectedVisualFeature {
            pixel: point![40000.0 + i as f32 * 10.0, 40000.0],
            confidence: 0.95,
        }));
    assert_eq!(features.supported_feature_count(), 33);
    assert!(
        associate_tracking_visual_features(input(&features, &geometry), &parameters)
            .associations
            .is_empty()
    );
}

#[test]
#[ignore = "host runtime characterization; run explicitly with --ignored --nocapture"]
fn runtime_characterization() {
    use std::{hint::black_box, time::Instant};

    let mut geometry = geometry();
    let map = LandmarkMap::new(
        &FieldDimensions::SPL_2025,
        GlobalAssociationConfig::default().symmetry_epsilon,
    );
    let sparse = (3..=5)
        .map(|n| {
            project(
                stationary_three().into_iter().chain(
                    map.landmarks
                        .iter()
                        .filter(|landmark| landmark.class == VisualFeatureClass::LSpot)
                        .take(n - 3)
                        .map(|landmark| (landmark.class, landmark.xy)),
                ),
                &geometry,
            )
        })
        .collect::<Vec<_>>();
    let rich = project(map.landmarks.iter().map(|l| (l.class, l.xy)), &geometry);
    let parameters = calibrated_parameters();
    let mut high_covariance = tracking_estimate(0.0);
    high_covariance.covariance = sparse_retained_covariance();
    for (mode, estimate) in [
        ("global", tracking_estimate(0.0)),
        ("tracking", tracking_estimate(1.0e-6)),
        ("tracking-high-covariance", high_covariance),
    ] {
        geometry.estimate = estimate;
        for (frame, features) in [
            ("3", &sparse[0]),
            ("4", &sparse[1]),
            ("5", &sparse[2]),
            ("rich", &rich),
        ] {
            if mode == "tracking-high-covariance" && frame == "rich" {
                continue;
            }
            let mut input = input(features, &geometry);
            if mode == "tracking-high-covariance" {
                input.time = input.time + std::time::Duration::from_millis(3100);
            }
            let mut samples = Vec::with_capacity(500);
            for iteration in 0..520 {
                let started = Instant::now();
                let result = black_box(if mode == "global" {
                    associate_global_visual_features(global_input(
                        black_box(input),
                        black_box(&parameters.global_localizer),
                    ))
                } else {
                    associate_tracking_visual_features(black_box(input), black_box(&parameters))
                });
                let elapsed = started.elapsed();
                assert_eq!(
                    result.associations.len(),
                    features.supported_feature_count()
                );
                if iteration >= 20 {
                    samples.push(elapsed);
                }
            }
            samples.sort_unstable();
            eprintln!(
                "{mode}/{frame}: n={} p50={:?} p95={:?} p99={:?} max={:?}",
                samples.len(),
                samples[250],
                samples[475],
                samples[495],
                samples[499]
            );
        }
    }
}
