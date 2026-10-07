use coordinate_systems::{Camera, Field, Pixel, Robot};
use fagra::{
    BlockId, EvaluationError, FactorBatch, FactorSelection, JacobianBlock, LinearizationSink,
    StateKey, StateStore,
};
use linear_algebra::{Isometry3, Point2, Point3};
use nalgebra::{Matrix3, RealField, SMatrix};

use super::common;
use crate::variables::{CameraIntrinsics, FieldAlignment, PoseControl, rotation::scalar};

/// A detected image point associated with a known, fixed field landmark.
#[derive(Clone, Debug)]
pub struct ReprojectionObservation<R: RealField + Copy = f64> {
    pub field_point: Point3<Field, R>,
    pub detection: Point2<Pixel, R>,
}

/// A frame's monotone oriented-bearing residuals share trajectory and camera geometry.
///
/// For predicted unit bearing `d`, observed unit bearing `b`, and `c = b.dot(d)`,
/// the three-component residual is `sqrt(3 / (2 + c)) * (d - b) / sigma_theta`.
/// Its least-squares cost is `3 * (1 - c) / ((2 + c) * sigma_theta^2)`, strictly
/// increasing with angular error on `(0, pi)`. Evaluation is smooth across optical
/// z=0 and behind the camera. Only zero/near-zero range and invalid inputs fail.
///
/// Huber acts on the norm of each complete whitened 3D residual. Both the observed
/// bearing and the directional weight are differentiated, including intrinsics.
/// The exact antipode is a stationary maximum; this is not a global convergence
/// guarantee. Final visibility and pixel-error checks belong to the caller.
/// The historical type name is retained; this is NOT a pixel-reprojection likelihood.
#[derive(Clone, Debug)]
pub struct FrameReprojections<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub alignment: StateKey<FieldAlignment<R>>,
    pub intrinsics: StateKey<CameraIntrinsics<R>>,
    pub duration: R,
    pub tau: R,
    /// Optical convention: x right, y down, z forward, before perspective division.
    pub robot_to_camera: Isometry3<Robot, Camera, R>,
    /// Fixed inverse angular standard deviation (radians^-1), independent of the
    /// optimized intrinsics. Isotropic angular noise, not anisotropic pixel noise.
    pub angular_information_root: R,
    pub huber_threshold: R,
    /// Positive minimum camera-to-landmark range in metres, NOT optical depth.
    pub min_range: R,
}

impl<R, S> FactorBatch<S> for FrameReprojections<R>
where
    R: RealField + Copy,
    S: StateStore<PoseControl<R>> + StateStore<FieldAlignment<R>> + StateStore<CameraIntrinsics<R>>,
{
    type Scalar = R;
    type Factor = ReprojectionObservation<R>;

    fn visit_variables(&self, _factor: &Self::Factor, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
        visitor(self.alignment.block_id());
        visitor(self.intrinsics.block_id());
    }

    fn cost(
        &self,
        states: &S,
        factors: FactorSelection<'_, Self::Factor>,
    ) -> Result<R, EvaluationError> {
        if factors.is_empty() {
            return Ok(R::zero());
        }
        self.validate()?;
        let pose = common::spline(states, &self.controls, self.duration)?.pose(self.tau)?;
        let geometry = Geometry::new(
            &pose.inner,
            states.get(self.alignment)?,
            states.get(self.intrinsics)?,
            &self.robot_to_camera.inner,
        )?;
        let mut cost = R::zero();
        for (_, observation) in factors {
            let projection = geometry.bearing(observation, self.min_range)?;
            cost += common::huber(
                &(projection.error * self.angular_information_root),
                self.huber_threshold,
            )?
            .0;
        }
        common::checked_cost(cost)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        factors: FactorSelection<'_, Self::Factor>,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        if factors.is_empty() {
            return Ok(());
        }
        self.validate()?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let pose = spline.linearize()?.pose(self.tau)?;
        let geometry = Geometry::new(
            &pose.pose.inner,
            states.get(self.alignment)?,
            states.get(self.intrinsics)?,
            &self.robot_to_camera.inner,
        )?;
        let robot_to_camera = geometry
            .robot_to_camera
            .rotation
            .to_rotation_matrix()
            .into_inner();
        let local_to_robot = geometry
            .local_to_robot
            .rotation
            .to_rotation_matrix()
            .into_inner();
        for (id, observation) in factors {
            let projection = geometry.bearing(observation, self.min_range)?;
            let residual = projection.error * self.angular_information_root;
            let scale = common::huber(&residual, self.huber_threshold)?.1;
            let d = projection.predicted;
            let b = projection.observed;
            let difference = d - b;
            let denominator = scalar::<R>(2.0) + b.dot(&d);
            let weight_derivative = scalar::<R>(0.5) / denominator;
            let root = self.angular_information_root * scale * projection.weight;
            let predicted_jacobian = (Matrix3::identity()
                - difference * b.transpose() * weight_derivative)
                * ((Matrix3::identity() - d * d.transpose()) / projection.range);
            let camera = predicted_jacobian * root * robot_to_camera;
            let mut inverse_pose = SMatrix::<R, 3, 6>::zeros();
            inverse_pose
                .fixed_view_mut::<3, 3>(0, 0)
                .copy_from(&projection.robot.cross_matrix());
            inverse_pose
                .fixed_view_mut::<3, 3>(0, 3)
                .set_diagonal(&nalgebra::Vector3::repeat(-R::one()));
            let h_pose = camera * inverse_pose;
            let jacobians = pose.jacobians.map(|j| h_pose * j);
            // Right perturbation of alignment: q_local' = Exp(-delta) q_local.
            let alignment = SMatrix::<R, 3, 3>::new(
                projection.local.y,
                -R::one(),
                R::zero(),
                -projection.local.x,
                R::zero(),
                -R::one(),
                R::zero(),
                R::zero(),
                R::zero(),
            );
            let alignment = camera * local_to_robot * alignment;
            let ray = projection.observed_ray;
            let ray_jacobian = SMatrix::<R, 3, 4>::new(
                -ray.x / geometry.fx,
                R::zero(),
                -geometry.fx.recip(),
                R::zero(),
                R::zero(),
                -ray.y / geometry.fy,
                R::zero(),
                -geometry.fy.recip(),
                R::zero(),
                R::zero(),
                R::zero(),
                R::zero(),
            );
            let intrinsics = (-Matrix3::identity()
                - difference * d.transpose() * weight_derivative)
                * ((Matrix3::identity() - b * b.transpose()) / projection.ray_norm)
                * ray_jacobian
                * root;
            let blocks: [_; 6] = std::array::from_fn(|i| match i {
                0..=3 => JacobianBlock::new(self.controls[i], &jacobians[i]),
                4 => JacobianBlock::new(self.alignment, &alignment),
                _ => JacobianBlock::new(self.intrinsics, &intrinsics),
            });
            sink.factor(id, |sink| sink.residual(&(residual * scale), &blocks))?;
        }
        Ok(())
    }
}

impl<R: RealField + Copy> FrameReprojections<R> {
    fn validate(&self) -> Result<(), EvaluationError> {
        common::positive(self.min_range)?;
        common::positive(self.huber_threshold)?;
        common::positive(self.angular_information_root)?;
        Ok(())
    }
}

struct Geometry<R: RealField + Copy> {
    field_to_local: nalgebra::Isometry2<R>,
    local_to_robot: nalgebra::Isometry3<R>,
    robot_to_camera: nalgebra::Isometry3<R>,
    fx: R,
    fy: R,
    cx: R,
    cy: R,
}

struct Projection<R: RealField + Copy> {
    local: nalgebra::Vector3<R>,
    robot: nalgebra::Vector3<R>,
    predicted: nalgebra::Vector3<R>,
    observed: nalgebra::Vector3<R>,
    observed_ray: nalgebra::Vector3<R>,
    range: R,
    ray_norm: R,
    weight: R,
    error: nalgebra::Vector3<R>,
}

impl<R: RealField + Copy> Geometry<R> {
    fn new(
        pose: &nalgebra::Isometry3<R>,
        alignment: &FieldAlignment<R>,
        intrinsics: &CameraIntrinsics<R>,
        extrinsic: &nalgebra::Isometry3<R>,
    ) -> Result<Self, EvaluationError> {
        common::finite(
            alignment
                .local_to_field
                .inner
                .translation
                .vector
                .iter()
                .chain(
                    [
                        alignment.local_to_field.inner.rotation.re,
                        alignment.local_to_field.inner.rotation.im,
                    ]
                    .iter(),
                )
                .chain(extrinsic.translation.vector.iter())
                .chain(extrinsic.rotation.coords.iter())
                .chain(intrinsics.optical_center.inner.coords.iter()),
        )?;
        let fx = intrinsics.focal_lengths.inner.x;
        let fy = intrinsics.focal_lengths.inner.y;
        common::positive(fx)?;
        common::positive(fy)?;
        Ok(Self {
            field_to_local: alignment.local_to_field.inner.inverse(),
            local_to_robot: pose.inverse(),
            robot_to_camera: *extrinsic,
            fx,
            fy,
            cx: intrinsics.optical_center.inner.x,
            cy: intrinsics.optical_center.inner.y,
        })
    }

    fn bearing(
        &self,
        observation: &ReprojectionObservation<R>,
        min_range: R,
    ) -> Result<Projection<R>, EvaluationError> {
        common::finite(
            observation
                .field_point
                .inner
                .coords
                .iter()
                .chain(observation.detection.inner.coords.iter()),
        )?;
        let xy =
            self.field_to_local * nalgebra::Point2::from(observation.field_point.inner.coords.xy());
        let local = nalgebra::Vector3::new(xy.x, xy.y, observation.field_point.inner.z);
        let robot = (self.local_to_robot * nalgebra::Point3::from(local)).coords;
        let camera = (self.robot_to_camera * nalgebra::Point3::from(robot)).coords;
        let range = camera.norm();
        if !range.is_finite() || range <= min_range {
            return Err(EvaluationError::InvalidEvaluation);
        }
        let predicted = camera / range;
        let observed_ray = nalgebra::Vector3::new(
            (observation.detection.inner.x - self.cx) / self.fx,
            (observation.detection.inner.y - self.cy) / self.fy,
            R::one(),
        );
        let ray_norm = observed_ray.norm();
        if !ray_norm.is_finite() {
            return Err(EvaluationError::InvalidEvaluation);
        }
        let observed = observed_ray / ray_norm;
        let weight = (scalar::<R>(3.0) / (scalar::<R>(2.0) + observed.dot(&predicted))).sqrt();
        let error = (predicted - observed) * weight;
        common::finite(error.iter())?;
        Ok(Projection {
            local,
            robot,
            predicted,
            observed,
            observed_ray,
            range,
            ray_norm,
            weight,
            error,
        })
    }
}
