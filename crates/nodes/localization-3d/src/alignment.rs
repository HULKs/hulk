use coordinate_systems::{Camera, Field, Local, Robot};
use linear_algebra::{IntoTransform, Isometry2, Isometry3, Orientation2, Rotation3, Vector3};
use localization_fagra::alignment::{GravityCamera, fit_ground_similarity};
use projection::intrinsic::Intrinsic;
use types::localization::HeadingConstraint;
use types::visual_localization::FieldMarkAssociation;
use types::visual_localization::VisualLocalizationFrame;

use crate::parameters::VisualParameters;

pub(crate) fn valid_visual_rms(rms: Option<f64>, parameters: &VisualParameters) -> bool {
    rms.is_some_and(|rms| rms.is_finite() && rms <= parameters.max_rms_px)
}

pub(crate) fn reprojection_rms(
    associations: &[FieldMarkAssociation],
    field_to_camera: &nalgebra::Isometry3<f64>,
    focals: nalgebra::Vector2<f64>,
    optical_center: nalgebra::Point2<f64>,
    parameters: &VisualParameters,
) -> Option<f64> {
    let mut squared = 0.0;
    for observation in associations {
        let p = field_to_camera * observation.field_point.inner.cast::<f64>();
        if p.z <= parameters.min_reprojection_depth {
            return None;
        }
        let pixel = nalgebra::Vector2::new(
            focals.x * p.x / p.z + optical_center.x,
            focals.y * p.y / p.z + optical_center.y,
        );
        squared += (pixel - observation.detection.inner.coords.cast::<f64>()).norm_squared();
    }
    let rms = (squared / associations.len() as f64).sqrt();
    rms.is_finite().then_some(rms)
}

pub(crate) fn seed_recovery_alignment(
    frame: &mut VisualLocalizationFrame,
    robot_to_local: Rotation3<Robot, Local>,
    heading: HeadingConstraint,
    parameters: &VisualParameters,
) -> Option<(Isometry3<Robot, Local>, Isometry2<Local, Field>)> {
    let (pose, mut alignment) = seed_alignment(
        robot_to_local,
        frame.robot_to_camera,
        frame.camera_intrinsic,
        &mut frame.associations,
        false,
        parameters,
    )?;
    let field_heading = Orientation2::new(
        (alignment.to_3d().inner.rotation * pose.inner.rotation)
            .euler_angles()
            .2 as f64,
    );
    let error = heading
        .expected
        .rotation_to(field_heading)
        .inner
        .angle()
        .abs();
    let flipped = error > std::f64::consts::FRAC_PI_2;
    let error = if flipped {
        std::f64::consts::PI - error
    } else {
        error
    };
    if !heading.is_valid() || !error.is_finite() || error > heading.max_error {
        return None;
    }
    if flipped {
        alignment.inner =
            nalgebra::Isometry2::new(nalgebra::Vector2::zeros(), std::f32::consts::PI)
                * alignment.inner;
        for association in &mut frame.associations {
            association.field_point.inner.x = -association.field_point.inner.x;
            association.field_point.inner.y = -association.field_point.inner.y;
        }
    }
    Some((pose, alignment))
}

pub(crate) fn valid_visual_frame(
    frame: &types::visual_localization::VisualLocalizationFrame,
    parameters: &VisualParameters,
) -> bool {
    let associations = &frame.associations;
    (parameters.min_associations..=parameters.max_associations).contains(&associations.len())
        && frame.camera_intrinsic.is_valid()
        && frame
            .robot_to_camera
            .inner
            .to_homogeneous()
            .iter()
            .all(|value| value.is_finite())
        && associations.iter().all(|association| {
            association
                .detection
                .inner
                .iter()
                .chain(association.field_point.inner.iter())
                .all(|value| value.is_finite())
        })
        && associations.iter().enumerate().all(|(index, association)| {
            associations[index + 1..].iter().all(|other| {
                (association.detection - other.detection).inner.norm()
                    > parameters.min_detection_separation_px
                    && (association.field_point - other.field_point).inner.norm()
                        > parameters.min_landmark_separation_m
            })
        })
}

/// Fit camera height and field alignment from IMU tilt and bearings.
/// Startup canonicalizes the field half; recovery selects it from trusted heading.
pub(crate) fn seed_alignment(
    robot_to_local: Rotation3<Robot, Local>,
    robot_to_camera: Isometry3<Robot, Camera>,
    intrinsic: Intrinsic,
    associations: &mut [FieldMarkAssociation],
    canonicalize: bool,
    parameters: &VisualParameters,
) -> Option<(Isometry3<Robot, Local>, Isometry2<Local, Field>)> {
    let rotation: Rotation3<Robot, Local, f64> = Rotation3::wrap(robot_to_local.inner.cast());
    let camera_to_robot: Isometry3<Camera, Robot, f64> =
        Isometry3::wrap(robot_to_camera.inner.cast::<f64>()).inverse();
    let camera_rotation = rotation * camera_to_robot.rotation();
    let mut points = Vec::with_capacity(associations.len());
    for a in associations.iter() {
        let ray =
            camera_rotation * Vector3::wrap(intrinsic.bearing(a.detection).inner.cast::<f64>());
        if !ray.inner.iter().all(|v| v.is_finite()) || ray.z() >= -parameters.min_downward_ray {
            return None;
        }
        points.push((
            -ray.inner.xy() / ray.z(),
            a.field_point.inner.coords.xy().cast::<f64>(),
        ));
    }
    let fit = fit_ground_similarity(points.into_iter()).ok()?;
    let offset = rotation * camera_to_robot.translation().coords();
    let camera = GravityCamera::new(
        camera_rotation.inner.to_rotation_matrix().into_inner(),
        intrinsic.focals.cast(),
        intrinsic.optical_center.inner.coords.cast(),
        parameters.min_reprojection_depth,
    )?;
    let observations = associations
        .iter()
        .map(|a| {
            (
                a.field_point.inner.coords.xy().cast::<f64>(),
                a.detection.inner.coords.cast::<f64>(),
            )
        })
        .collect::<Vec<_>>();
    let fit = camera.refine(fit, &observations, |pose| pose.height > offset.z())?;
    let fitted_rotation = nalgebra::UnitComplex::new(fit.yaw);
    let body_height = fit.height - offset.z();
    if body_height <= 0.0 {
        return None;
    }
    let pose: Isometry3<Robot, Local> = nalgebra::Isometry3::from_parts(
        nalgebra::Translation3::new(0.0, 0.0, body_height),
        rotation.inner,
    )
    .cast()
    .framed_transform();
    let mut alignment = nalgebra::Isometry2::from_parts(
        (fit.position - fitted_rotation * offset.inner.xy()).into(),
        fitted_rotation,
    );
    let framed_alignment: Isometry2<Local, Field, f64> = alignment.framed_transform();
    let field_to_camera = robot_to_camera.inner.cast::<f64>()
        * pose.inner.cast::<f64>().inverse()
        * framed_alignment.to_3d().inner.inverse();
    if !valid_visual_rms(
        reprojection_rms(
            associations,
            &field_to_camera,
            intrinsic.focals.cast(),
            intrinsic.optical_center.inner.cast(),
            parameters,
        ),
        parameters,
    ) {
        return None;
    }
    if canonicalize && alignment.translation.vector.x > 0.0 {
        alignment =
            nalgebra::Isometry2::new(nalgebra::Vector2::zeros(), std::f64::consts::PI) * alignment;
        for association in associations {
            association.field_point.inner.x = -association.field_point.inner.x;
            association.field_point.inner.y = -association.field_point.inner.y;
        }
    }
    let alignment = alignment.cast::<f32>();
    alignment
        .to_homogeneous()
        .iter()
        .all(|value| value.is_finite())
        .then(|| (pose, alignment.framed_transform()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::point;

    #[test]
    fn recorded_ground_fit_is_refined_before_recovery_pixel_validation() {
        let frame: serde_json::Value = serde_json::from_str(include_str!(
            "../../field_mark_association/src/global_association/recorded_frame.json"
        ))
        .unwrap();
        let mut associations = frame["detections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| FieldMarkAssociation {
                detection: serde_json::from_value(d["pixel"].clone()).unwrap(),
                field_point: point![
                    d["field"][0].as_f64().unwrap() as f32,
                    d["field"][1].as_f64().unwrap() as f32,
                    0.0
                ],
            })
            .collect::<Vec<_>>();
        let camera: Isometry3<Robot, Camera> =
            serde_json::from_value(frame["robot_to_camera"].clone()).unwrap();
        let intrinsic: Intrinsic = serde_json::from_value(frame["intrinsics"].clone()).unwrap();
        let parameters = VisualParameters::default();
        let (pose, alignment) = seed_alignment(
            Rotation3::from_euler_angles(
                frame["roll"].as_f64().unwrap() as f32,
                frame["pitch"].as_f64().unwrap() as f32,
                0.0,
            ),
            camera,
            intrinsic,
            &mut associations,
            false,
            &parameters,
        )
        .unwrap();
        let transform = (camera * pose.inverse() * alignment.to_3d().inverse())
            .inner
            .cast::<f64>();
        let rms = reprojection_rms(
            &associations,
            &transform,
            intrinsic.focals.cast(),
            intrinsic.optical_center.inner.cast(),
            &parameters,
        )
        .unwrap();
        assert!((rms - 4.146).abs() < 0.01, "pixel RMS {rms}");
        assert!((pose.translation().z() - 0.448).abs() < 0.01);
    }

    #[test]
    fn visual_gates_change_admission_and_pixel_validation() {
        let mut parameters = VisualParameters::default();
        let frame = VisualLocalizationFrame {
            epoch: 0,
            generation: 0,
            source: types::visual_localization::VisualAssociationSource::Tracking,
            robot_to_camera: Isometry3::identity(),
            camera_intrinsic: Intrinsic::default(),
            associations: vec![
                FieldMarkAssociation {
                    field_point: point![0.0, 0.0, 2.0],
                    detection: point![0.0, 0.0],
                },
                FieldMarkAssociation {
                    field_point: point![4.0, 0.0, 2.0],
                    detection: point![2.0, 0.0],
                },
                FieldMarkAssociation {
                    field_point: point![0.0, 4.0, 2.0],
                    detection: point![0.0, 2.0],
                },
            ],
        };
        assert!(valid_visual_frame(&frame, &parameters));
        parameters.min_associations = 4;
        assert!(!valid_visual_frame(&frame, &parameters));
        parameters.min_associations = 3;
        parameters.min_detection_separation_px = 2.1;
        assert!(!valid_visual_frame(&frame, &parameters));
        parameters.min_detection_separation_px = 1.0;
        parameters.min_landmark_separation_m = 4.1;
        assert!(!valid_visual_frame(&frame, &parameters));
        let rms = |parameters: &VisualParameters| {
            reprojection_rms(
                &frame.associations,
                &nalgebra::Isometry3::identity(),
                nalgebra::Vector2::repeat(1.0),
                nalgebra::Point2::new(1.0, 0.0),
                parameters,
            )
        };
        assert!(valid_visual_rms(rms(&parameters), &parameters));
        parameters.max_rms_px = 0.5;
        assert!(!valid_visual_rms(rms(&parameters), &parameters));
        parameters.min_reprojection_depth = 2.1;
        assert!(rms(&parameters).is_none());
    }
}
