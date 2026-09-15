//! Analytic gaze geometry for K1. Target selection and motor control live elsewhere.

use std::f64::consts::TAU;

use coordinate_systems::{Ground, Pixel, Robot};
use kinematics::{joints::head::HeadJoints, robot_dimensions::RobotDimensions};
use linear_algebra::{Isometry3, Point2, Point3, nalgebra::Vector3, point};
use projection::camera_matrix::CameraMatrix;
use types::{motion_command::ImageRegion, parameters::ImageRegionParameters};

// Numerical tolerances, not a behavioral arrival criterion. The final forward check
// also guards roundoff when converting the analytic f64 solution into f32 commands.
const MINIMUM_DISTANCE: f64 = 1e-6;
const MAXIMUM_PIXEL_ERROR: f32 = 0.05;

pub struct GazeGeometry<'a> {
    pub camera_matrix: &'a CameraMatrix,
    pub ground_to_robot: Isometry3<Ground, Robot>,
}

struct RayGeometry {
    /// Vector from the pitch pivot to the target, expressed in Robot axes.
    pivot_target: Vector3<f64>,
    /// Left optical center expressed in Head coordinates.
    camera_origin: Vector3<f64>,
    /// Unit direction of the requested image ray, expressed in Head axes.
    camera_ray: Vector3<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookAtError {
    InvalidTarget,
    InvalidGeometry,
    InvalidReference,
    NoSolution,
}

/// Place a ground-relative point at the requested left-image position.
/// Height is measured along Ground's +Z axis, in meters (e.g. ball radius).
/// The reference chooses the nearest solution, including equivalent full turns;
/// use the current joint-control reference, or measurements before initialization.
/// Returned angles are unconstrained: joint control owns mechanical limits.
pub fn look_at(
    target: Point2<Ground>,
    height_above_ground: f32,
    image_region: ImageRegion,
    geometry: &GazeGeometry<'_>,
    parameters: &ImageRegionParameters,
    reference: HeadJoints<f32>,
) -> Result<HeadJoints<f32>, LookAtError> {
    let target = point![target.x(), target.y(), height_above_ground];
    if !target.inner.coords.iter().all(|value| value.is_finite()) {
        return Err(LookAtError::InvalidTarget);
    }
    if !reference.into_iter().all(f32::is_finite) {
        return Err(LookAtError::InvalidReference);
    }
    let pixel = requested_pixel(image_region, parameters, geometry)?;
    let RayGeometry {
        pivot_target,
        camera_origin,
        camera_ray,
    } = ray_geometry(target, pixel, geometry);
    let mut best = None;
    let mut best_distance = f64::INFINITY;

    for distance in
        ray_distances(pivot_target, camera_origin, camera_ray).ok_or(LookAtError::NoSolution)?
    {
        if distance <= MINIMUM_DISTANCE || !distance.is_finite() {
            continue;
        }
        let point_on_ray = camera_origin + distance * camera_ray;
        let Some(candidates) = joint_solutions(pivot_target, point_on_ray, reference) else {
            continue;
        };
        for candidate in candidates {
            if !frames_target(candidate, target, pixel, geometry) {
                continue;
            }
            let distance = (f64::from(candidate.yaw) - f64::from(reference.yaw)).powi(2)
                + (f64::from(candidate.pitch) - f64::from(reference.pitch)).powi(2);
            if distance < best_distance {
                best = Some(candidate);
                best_distance = distance;
            }
        }
    }
    best.ok_or(LookAtError::NoSolution)
}

fn requested_pixel(
    region: ImageRegion,
    parameters: &ImageRegionParameters,
    geometry: &GazeGeometry<'_>,
) -> Result<Point2<Pixel>, LookAtError> {
    let camera = geometry.camera_matrix;
    let normalized = match region {
        ImageRegion::Center => parameters.center,
        ImageRegion::Bottom => parameters.bottom,
        ImageRegion::Top => parameters.top,
    };
    let valid = normalized
        .inner
        .coords
        .iter()
        .all(|value| (0.0..=1.0).contains(value))
        && camera
            .image_size
            .inner
            .iter()
            .chain(camera.intrinsics.focals.iter())
            .all(|value| value.is_finite() && *value > 0.0)
        && camera
            .intrinsics
            .optical_center
            .inner
            .coords
            .iter()
            .all(|value| value.is_finite())
        && valid_transform(geometry.ground_to_robot)
        && valid_transform(camera.head_to_camera)
        && (camera.correction_in_robot.inner.quaternion().norm_squared() - 1.0).abs() < 1e-5;
    if !valid {
        return Err(LookAtError::InvalidGeometry);
    }
    Ok(point![
        normalized.x() * camera.image_size.x(),
        normalized.y() * camera.image_size.y()
    ])
}

fn valid_transform<From, To>(transform: Isometry3<From, To>) -> bool {
    transform
        .inner
        .translation
        .vector
        .iter()
        .all(|value| value.is_finite())
        && (transform.inner.rotation.quaternion().norm_squared() - 1.0).abs() < 1e-5
}

fn ray_geometry(
    target: Point3<Ground>,
    pixel: Point2<Pixel>,
    geometry: &GazeGeometry<'_>,
) -> RayGeometry {
    let camera = geometry.camera_matrix;
    let target_in_robot = camera.correction_in_robot * (geometry.ground_to_robot * target);
    // K1's pitch pivot lies on the yaw axis, so its position is independent of yaw.
    let pivot = RobotDimensions::ROBOT_TO_NECK.inner + RobotDimensions::NECK_TO_HEAD.inner;
    let pivot_target = target_in_robot.inner.coords.cast::<f64>() - pivot.cast::<f64>();
    let camera_to_head = camera.head_to_camera.inner.cast::<f64>().inverse();
    let camera_origin = camera_to_head.translation.vector;
    let camera_ray = (camera_to_head.rotation
        * Vector3::new(
            (f64::from(pixel.x()) - f64::from(camera.intrinsics.optical_center.x()))
                / f64::from(camera.intrinsics.focals.x),
            (f64::from(pixel.y()) - f64::from(camera.intrinsics.optical_center.y()))
                / f64::from(camera.intrinsics.focals.y),
            1.0,
        ))
    .normalize();
    RayGeometry {
        pivot_target,
        camera_origin,
        camera_ray,
    }
}

/// Rotations preserve distance to the pivot. Intersect c + d*r with the sphere
/// of radius |target|: d² + 2(c·r)d + |c|² - |target|² = 0, with |r| = 1.
fn ray_distances(
    target: Vector3<f64>,
    origin: Vector3<f64>,
    ray: Vector3<f64>,
) -> Option<[f64; 2]> {
    let along = origin.dot(&ray);
    let constant = origin.norm_squared() - target.norm_squared();
    let discriminant = along * along - constant;
    if discriminant < 0.0 {
        return None;
    }
    // Stable quadratic roots avoid cancellation when a target is close to the camera.
    let root = -along - discriminant.sqrt().copysign(along);
    if root == 0.0 {
        Some([0.0; 2])
    } else {
        Some([root, constant / root])
    }
}

/// Solve target = Rz(yaw) * Ry(pitch) * point. Pitch preserves the Y coordinate;
/// yaw preserves height. Both possible signs of the intermediate X are considered.
fn joint_solutions(
    target: Vector3<f64>,
    point: Vector3<f64>,
    reference: HeadJoints<f32>,
) -> Option<[HeadJoints<f32>; 2]> {
    let horizontal_squared = target.x * target.x + target.y * target.y;
    let x_squared = horizontal_squared - point.y * point.y;
    let roundoff = 1e-12 * target.norm_squared().max(point.norm_squared());
    if x_squared < -roundoff {
        return None;
    }
    Some(
        [x_squared.max(0.0).sqrt(), -x_squared.max(0.0).sqrt()].map(|x| {
            let yaw = if horizontal_squared <= MINIMUM_DISTANCE.powi(2) {
                f64::from(reference.yaw)
            } else {
                target.y.atan2(target.x) - point.y.atan2(x)
            };
            let pitch = if point.x.hypot(point.z) <= MINIMUM_DISTANCE {
                f64::from(reference.pitch)
            } else {
                point.z.atan2(point.x) - target.z.atan2(x)
            };
            HeadJoints {
                yaw: nearest_equivalent(yaw, reference.yaw),
                pitch: nearest_equivalent(pitch, reference.pitch),
            }
        }),
    )
}

fn nearest_equivalent(angle: f64, reference: f32) -> f32 {
    (angle + TAU * ((f64::from(reference) - angle) / TAU).round()) as f32
}

fn frames_target(
    joints: HeadJoints<f32>,
    target: Point3<Ground>,
    pixel: Point2<Pixel>,
    geometry: &GazeGeometry<'_>,
) -> bool {
    let camera = geometry.camera_matrix;
    let camera_target = camera.ground_to_left_camera_at(&joints, geometry.ground_to_robot) * target;
    if camera_target.z() <= MINIMUM_DISTANCE as f32 {
        return false;
    }
    let projected = camera.intrinsics.project(camera_target.coords());
    (projected - pixel).norm() <= MAXIMUM_PIXEL_ERROR
}

#[cfg(test)]
mod tests {
    use kinematics::forward::{head_to_left_camera, head_to_robot};
    use linear_algebra::{Orientation3, Rotation3, nalgebra, vector};

    use super::*;

    fn camera() -> CameraMatrix {
        CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.55, 0.65],
            nalgebra::point![0.48, 0.52],
            vector![640.0, 480.0],
            Isometry3::identity(),
            head_to_robot(&HeadJoints {
                yaw: -0.3,
                pitch: 0.2,
            })
            .inverse(),
            head_to_left_camera(-0.2),
        )
        .to_corrected(
            Rotation3::from_euler_angles(0.03, -0.04, 0.02),
            Rotation3::from_euler_angles(-0.02, 0.01, 0.03),
        )
    }

    fn regions() -> ImageRegionParameters {
        ImageRegionParameters {
            center: point![0.5, 0.5],
            top: point![0.45, 0.15],
            bottom: point![0.55, 0.85],
        }
    }

    #[test]
    fn targets_and_framing_roundtrip_through_the_full_camera_chain() {
        let camera = camera();
        let geometry = GazeGeometry {
            camera_matrix: &camera,
            ground_to_robot: Isometry3::from_parts(
                vector![0.05, -0.03, -0.55],
                Orientation3::from_euler_angles(0.1, -0.08, 0.04),
            ),
        };
        let regions = regions();
        // Include near targets, substantial yaw, targets behind the initial camera,
        // and elevated points. Neither robot tilt nor calibration may be discarded.
        for x in [-2.0, 0.35, 0.8, 2.0, 5.0] {
            for y in [-1.0, 0.0, 1.0] {
                for height in [0.0, 0.105, 0.3] {
                    for (region, desired) in [
                        (ImageRegion::Center, regions.center),
                        (ImageRegion::Top, regions.top),
                        (ImageRegion::Bottom, regions.bottom),
                    ] {
                        let joints = look_at(
                            point![x, y],
                            height,
                            region,
                            &geometry,
                            &regions,
                            HeadJoints::default(),
                        )
                        .unwrap();
                        let target_in_camera = camera
                            .ground_to_left_camera_at(&joints, geometry.ground_to_robot)
                            * point![x, y, height];
                        assert!(target_in_camera.z() > 0.0);
                        let pixel = camera.intrinsics.project(target_in_camera.coords());
                        let expected = point![desired.x() * 640.0, desired.y() * 480.0];
                        assert!(
                            (pixel - expected).norm() < 0.01,
                            "{x}, {y}, {height}, {region:?}: {pixel:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn nearby_reference_keeps_the_same_solution_across_the_yaw_wrap() {
        let camera = camera();
        let geometry = GazeGeometry {
            camera_matrix: &camera,
            ground_to_robot: Isometry3::from_translation(0.0, 0.0, -0.55),
        };
        let mut reference = HeadJoints {
            yaw: 3.1,
            pitch: 0.3,
        };
        for y in [-0.01, 0.0, 0.01] {
            let joints = look_at(
                point![-2.0, y],
                0.105,
                ImageRegion::Center,
                &geometry,
                &regions(),
                reference,
            )
            .unwrap();
            assert!((joints.yaw - reference.yaw).abs() < 0.1);
            assert!(
                joints.pitch.abs() < 1.0,
                "must not switch to the upside-down solution"
            );
            reference = joints;
        }
    }

    #[test]
    fn invalid_inputs_and_impossible_geometry_return_errors() {
        let mut camera = camera();
        let regions = regions();
        let solve = |camera: &CameraMatrix, target, height, reference| {
            look_at(
                target,
                height,
                ImageRegion::Center,
                &GazeGeometry {
                    camera_matrix: camera,
                    ground_to_robot: Isometry3::identity(),
                },
                &regions,
                reference,
            )
        };
        assert_eq!(
            solve(&camera, point![f32::NAN, 0.0], 0.0, HeadJoints::default()),
            Err(LookAtError::InvalidTarget)
        );
        assert_eq!(
            solve(
                &camera,
                point![1.0, 0.0],
                f32::INFINITY,
                HeadJoints::default()
            ),
            Err(LookAtError::InvalidTarget)
        );
        assert_eq!(
            solve(&camera, point![1.0, 0.0], 0.0, HeadJoints::fill(f32::NAN)),
            Err(LookAtError::InvalidReference)
        );
        camera.intrinsics.focals.x = 0.0;
        assert_eq!(
            solve(&camera, point![1.0, 0.0], 0.0, HeadJoints::default()),
            Err(LookAtError::InvalidGeometry)
        );

        // With the camera pointing outward, the pitch pivot cannot lie on its
        // forward center ray at any yaw/pitch. Do not fabricate an angle for it.
        camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
            Isometry3::identity(),
            head_to_robot(&HeadJoints::default()).inverse(),
            head_to_left_camera(0.0),
        );
        assert_eq!(
            solve(
                &camera,
                point![0.0056, 0.0],
                0.2149 + 0.033,
                HeadJoints::default()
            ),
            Err(LookAtError::NoSolution)
        );
    }
}
