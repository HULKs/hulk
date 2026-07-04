use ::types::field_dimensions::{FieldDimensions, Half, Side};
use coordinate_systems::{Camera, Field, Ground, Pixel, Robot};
use linear_algebra::{IntoTransform, Isometry3, Point2, point};
use projection::intrinsic::Intrinsic;

use super::*;
use crate::{DetectedVisualFeature, DetectedVisualFeatures};

fn camera_intrinsic() -> Intrinsic {
    Intrinsic::new(nalgebra::vector![100.0, 100.0], point![320.0, 240.0])
}

fn robot_to_camera() -> Isometry3<Robot, Camera> {
    nalgebra::Isometry3::translation(0.0, 0.0, 0.5).framed_transform()
}

fn input<'a>(
    visual_features: &'a DetectedVisualFeatures,
    field_dimensions: &'a FieldDimensions,
    config_hint: Option<Isometry3<Robot, Field>>,
) -> GlobalLocalizationInput<'a> {
    GlobalLocalizationInput {
        visual_features,
        field_dimensions,
        ground_to_robot: Isometry3::<Ground, Robot>::identity(),
        robot_to_camera: robot_to_camera(),
        camera_intrinsic: camera_intrinsic(),
        pose_hint: config_hint,
    }
}

fn project_point(point: Point2<Field>) -> DetectedVisualFeature {
    let field_to_camera = robot_to_camera() * Isometry3::<Field, Robot>::identity();
    let pixel = camera_intrinsic().project((field_to_camera * point.extend(0.0)).coords());
    DetectedVisualFeature {
        pixel,
        confidence: 0.9,
    }
}

fn feature_with_confidence(point: Point2<Field>, confidence: f32) -> DetectedVisualFeature {
    DetectedVisualFeature {
        confidence,
        ..project_point(point)
    }
}

fn synthetic_features(field: &FieldDimensions) -> DetectedVisualFeatures {
    DetectedVisualFeatures {
        goalposts: vec![
            project_point(field.goal_post(Half::Opponent, Side::Left)),
            project_point(field.goal_post(Half::Opponent, Side::Right)),
        ],
        l_spots: vec![project_point(field.corner(Half::Opponent, Side::Left))],
        x_spots: Vec::new(),
        t_spots: vec![project_point(field.t_crossing(Side::Left))],
        penalty_spots: vec![project_point(field.penalty_spot(Half::Opponent))],
    }
}

#[test]
fn pose_hint_fallback_associates_single_feature() {
    let field = FieldDimensions::SPL_2025;
    let penalty_spot = field.penalty_spot(Half::Opponent);
    let features = DetectedVisualFeatures {
        penalty_spots: vec![project_point(penalty_spot)],
        ..Default::default()
    };
    let localizer = GlobalAssociator::default();

    let associations = localizer.associate_with_pose_hint(
        input(
            &features,
            &field,
            Some(Isometry3::<Robot, Field>::identity()),
        ),
        PoseHintAssociationConfig::default(),
    );

    assert_eq!(associations.associations.len(), 1);
    assert_eq!(associations.associations[0].field_point, penalty_spot);
}

#[test]
fn pose_hint_fallback_uses_class_specific_confidence_thresholds() {
    let field = FieldDimensions::SPL_2025;
    let l_spot = field.corner(Half::Opponent, Side::Left);
    let penalty_spot = field.penalty_spot(Half::Opponent);
    let features = DetectedVisualFeatures {
        l_spots: vec![feature_with_confidence(l_spot, 0.2)],
        penalty_spots: vec![feature_with_confidence(penalty_spot, 0.2)],
        ..Default::default()
    };
    let localizer = GlobalAssociator::new(GlobalAssociationConfig {
        l_spot_confidence_threshold: 0.5,
        penalty_spot_confidence_threshold: 0.1,
        ..Default::default()
    });

    let associations = localizer.associate_with_pose_hint(
        input(
            &features,
            &field,
            Some(Isometry3::<Robot, Field>::identity()),
        ),
        PoseHintAssociationConfig::default(),
    );

    assert_eq!(associations.associations.len(), 1);
    assert_eq!(associations.associations[0].field_point, penalty_spot);
}

#[test]
fn pose_hint_fallback_rejects_second_best_tie() {
    let field = FieldDimensions::SPL_2025;
    let features = DetectedVisualFeatures {
        penalty_spots: vec![project_point(point![<Field>, 0.0, 0.0])],
        ..Default::default()
    };
    let localizer = GlobalAssociator::default();

    let associations = localizer.associate_with_pose_hint(
        input(
            &features,
            &field,
            Some(Isometry3::<Robot, Field>::identity()),
        ),
        PoseHintAssociationConfig {
            second_best_reprojection_margin_px: 10_000.0,
            ..Default::default()
        },
    );

    assert!(associations.associations.is_empty());
}

#[test]
fn pose_hint_fallback_assigns_shifted_xspots_as_group() {
    let field = FieldDimensions::SPL_2025;
    let center_pixel = project_point(field.center()).pixel;
    let right_pixel = project_point(field.x_crossing(Side::Right)).pixel;
    let y_shift = center_pixel.y() - right_pixel.y();
    let features = DetectedVisualFeatures {
        x_spots: vec![
            shifted_feature(field.x_crossing(Side::Right), y_shift),
            shifted_feature(field.center(), y_shift),
            shifted_feature(field.x_crossing(Side::Left), y_shift),
        ],
        ..Default::default()
    };
    let localizer = GlobalAssociator::default();

    let associations = localizer.associate_with_pose_hint(
        input(
            &features,
            &field,
            Some(Isometry3::<Robot, Field>::identity()),
        ),
        PoseHintAssociationConfig {
            max_reprojection_error_px: y_shift.abs() + 1.0,
            ..Default::default()
        },
    );

    assert_eq!(
        associations.associations.len(),
        3,
        "{:#?}",
        associations.associations
    );
    assert_associated(
        &associations.associations,
        features.x_spots[0].pixel,
        field.x_crossing(Side::Right),
    );
    assert_associated(
        &associations.associations,
        features.x_spots[1].pixel,
        field.center(),
    );
    assert_associated(
        &associations.associations,
        features.x_spots[2].pixel,
        field.x_crossing(Side::Left),
    );
}

fn shifted_feature(point: Point2<Field>, y_shift: f32) -> DetectedVisualFeature {
    let mut feature = project_point(point);
    feature.pixel = point![<Pixel>, feature.pixel.x(), feature.pixel.y() + y_shift];
    feature
}

fn assert_associated(
    associations: &[FeatureAssociation],
    detection: Point2<Pixel>,
    field_point: Point2<Field>,
) {
    assert!(associations.iter().any(|association| {
        (association.detection - detection).inner.norm() < 1.0e-4
            && (association.field_point - field_point).inner.norm() < 1.0e-4
    }));
}

#[test]
fn recovers_mixed_feature_associations_with_static_height_gate() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let features = synthetic_features(&field);
    let localizer = GlobalAssociator::default();

    let Some(result) = localizer.localize(input(
        &features,
        &field,
        Some(Isometry3::<Robot, Field>::identity()),
    )) else {
        return Err("synthetic features should localize".to_string());
    };

    assert!(result.is_unique());
    assert_eq!(result.associations().features.len(), 5);
    assert!(result.associations().score.reprojection_rmse < 1.0e-3);
    Ok(())
}

#[test]
fn rejects_static_height_outside_gate() {
    let field = FieldDimensions::SPL_2025;
    let features = synthetic_features(&field);
    let localizer = GlobalAssociator::new(GlobalAssociationConfig {
        height_max: 0.4,
        association_gate: 0.01,
        ..Default::default()
    });

    assert!(
        localizer
            .localize(input(
                &features,
                &field,
                Some(Isometry3::<Robot, Field>::identity())
            ))
            .is_none()
    );
}

#[test]
fn rejects_low_confidence_detections() {
    let field = FieldDimensions::SPL_2025;
    let mut features = synthetic_features(&field);
    for feature in features
        .goalposts
        .iter_mut()
        .chain(features.l_spots.iter_mut())
        .chain(features.t_spots.iter_mut())
        .chain(features.x_spots.iter_mut())
        .chain(features.penalty_spots.iter_mut())
    {
        feature.confidence = 0.1;
    }
    let localizer = GlobalAssociator::default();

    assert!(
        localizer
            .localize(input(
                &features,
                &field,
                Some(Isometry3::<Robot, Field>::identity())
            ))
            .is_none()
    );
}

#[test]
fn rejects_invalid_class_confidence_threshold() {
    let parameters = GlobalAssociationConfig {
        x_spot_confidence_threshold: 1.1,
        ..Default::default()
    };

    assert_eq!(
        parameters.validate(),
        Err(
            "global_localizer.x_spot_confidence_threshold must be finite and in [0, 1]".to_string()
        )
    );
}
