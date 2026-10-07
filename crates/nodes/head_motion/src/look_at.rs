//! Analytic look-at geometry for K1. Target selection and motor control live elsewhere.

use std::f32::consts::TAU;

use coordinate_systems::{Ground, Head, Pixel, Robot};
use kinematics::{forward::head_to_robot, joints::head::HeadJoints};
use linear_algebra::{Isometry3, Point2, Point3, Vector2, Vector3, point};
use projection::camera_matrix::CameraMatrix;
use types::{motion_command::ImageRegion, parameters::ImageRegionParameters};

// Numerical tolerances for solving and checking look-at geometry.
const MINIMUM_DISTANCE: f32 = 1e-6;
const MAXIMUM_PIXEL_ERROR: f32 = 0.05;

pub(crate) struct LookAtTarget {
    pub(crate) position: Point3<Ground>,
    pub(crate) image_region: ImageRegion,
}

pub(crate) struct LookAtGeometry {
    pub(crate) camera_matrix: CameraMatrix,
    pub(crate) ground_to_robot: Isometry3<Ground, Robot>,
}

impl LookAtGeometry {
    fn validate(&self) -> Result<(), LookAtError> {
        let camera = &self.camera_matrix;
        let valid = camera
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
            && valid_transform(self.ground_to_robot)
            && valid_transform(camera.head_to_camera)
            && (camera.correction_in_robot.inner.quaternion().norm_squared() - 1.0).abs() < 1e-5;
        if !valid {
            return Err(LookAtError::InvalidGeometry);
        }
        Ok(())
    }
}

struct RayGeometry {
    /// Vector from the pitch pivot to the target, expressed in Robot.
    pivot_target: Vector3<Robot>,
    camera_origin: Point3<Head>,
    /// Unit direction of the requested image ray, expressed in Head.
    camera_ray: Vector3<Head>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookAtError {
    InvalidTarget,
    InvalidGeometry,
    InvalidReferencePosition,
    NoSolution,
}

/// Place a ground-relative point at the requested image position.
/// The reference position chooses the nearest solution, including equivalent full turns;
/// use the last commanded position, or the measured position before initialization.
/// Returned angles are unconstrained: joint control owns mechanical limits.
pub(crate) fn look_at(
    target: &LookAtTarget,
    geometry: &LookAtGeometry,
    parameters: &ImageRegionParameters,
    reference_position: HeadJoints<f32>,
) -> Result<HeadJoints<f32>, LookAtError> {
    let position = target.position;
    if !position.inner.coords.iter().all(|value| value.is_finite()) {
        return Err(LookAtError::InvalidTarget);
    }
    if !reference_position.into_iter().all(f32::is_finite) {
        return Err(LookAtError::InvalidReferencePosition);
    }
    geometry.validate()?;
    let pixel = requested_pixel(
        target.image_region,
        parameters,
        geometry.camera_matrix.image_size,
    );
    let RayGeometry {
        pivot_target,
        camera_origin,
        camera_ray,
    } = ray_geometry(position, pixel, geometry);
    let mut best = None;
    let mut best_distance = f32::INFINITY;

    for distance in
        ray_distances(pivot_target, camera_origin, camera_ray).ok_or(LookAtError::NoSolution)?
    {
        if distance <= MINIMUM_DISTANCE || !distance.is_finite() {
            continue;
        }
        let point_on_ray = camera_origin + camera_ray * distance;
        let Some(candidates) = joint_solutions(pivot_target, point_on_ray, reference_position)
        else {
            continue;
        };
        for candidate in candidates {
            if !frames_target(candidate, position, pixel, geometry) {
                continue;
            }
            let distance = (candidate.yaw - reference_position.yaw).powi(2)
                + (candidate.pitch - reference_position.pitch).powi(2);
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
    image_size: Vector2<Pixel>,
) -> Point2<Pixel> {
    let normalized = match region {
        ImageRegion::Center => parameters.center,
        ImageRegion::Bottom => parameters.bottom,
        ImageRegion::Top => parameters.top,
    };
    point![
        normalized.x() * image_size.x(),
        normalized.y() * image_size.y()
    ]
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
    geometry: &LookAtGeometry,
) -> RayGeometry {
    let camera = &geometry.camera_matrix;
    let target_in_robot = camera.correction_in_robot * (geometry.ground_to_robot * target);
    // K1's pitch pivot lies on the yaw axis, so its position is independent of yaw.
    let pivot = head_to_robot(&HeadJoints::default()).translation();
    let pivot_target = target_in_robot - pivot;
    let camera_to_head = camera.head_to_camera.inverse();
    let camera_origin = camera_to_head.translation();
    let camera_ray = (camera_to_head * camera.intrinsics.bearing(pixel)).normalize();
    RayGeometry {
        pivot_target,
        camera_origin,
        camera_ray,
    }
}

/// Rotations preserve distance to the pivot. Intersect c + d*r with the sphere
/// of radius |target|: d² + 2(c·r)d + |c|² - |target|² = 0, with |r| = 1.
fn ray_distances(
    target: Vector3<Robot>,
    origin: Point3<Head>,
    ray: Vector3<Head>,
) -> Option<[f32; 2]> {
    let along = origin.coords().dot(&ray);
    let constant = origin.coords().norm_squared() - target.norm_squared();
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
    target: Vector3<Robot>,
    point: Point3<Head>,
    reference_position: HeadJoints<f32>,
) -> Option<[HeadJoints<f32>; 2]> {
    let horizontal_squared = target.x() * target.x() + target.y() * target.y();
    let x_squared = horizontal_squared - point.y() * point.y();
    let roundoff = 8.0 * f32::EPSILON * target.norm_squared().max(point.coords().norm_squared());
    if x_squared < -roundoff {
        return None;
    }
    Some(
        [x_squared.max(0.0).sqrt(), -x_squared.max(0.0).sqrt()].map(|x| {
            let yaw = if horizontal_squared <= MINIMUM_DISTANCE.powi(2) {
                reference_position.yaw
            } else {
                target.y().atan2(target.x()) - point.y().atan2(x)
            };
            let pitch = if point.x().hypot(point.z()) <= MINIMUM_DISTANCE {
                reference_position.pitch
            } else {
                point.z().atan2(point.x()) - target.z().atan2(x)
            };
            HeadJoints {
                yaw: nearest_equivalent(yaw, reference_position.yaw),
                pitch: nearest_equivalent(pitch, reference_position.pitch),
            }
        }),
    )
}

fn nearest_equivalent(angle: f32, reference_angle: f32) -> f32 {
    angle + TAU * ((reference_angle - angle) / TAU).round()
}

fn frames_target(
    candidate_position: HeadJoints<f32>,
    target: Point3<Ground>,
    pixel: Point2<Pixel>,
    geometry: &LookAtGeometry,
) -> bool {
    let camera = &geometry.camera_matrix;
    let camera_target =
        camera.ground_to_camera_at(&candidate_position, geometry.ground_to_robot) * target;
    if camera_target.z() <= MINIMUM_DISTANCE {
        return false;
    }
    let projected = camera.intrinsics.project(camera_target.coords());
    (projected - pixel).norm() <= MAXIMUM_PIXEL_ERROR
}
