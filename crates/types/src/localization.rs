use nalgebra::SMatrix;
use ros_z::time::Time;
use ros_z::{Message, MessageSchema, SchemaBuilder, SerdeCdrCodec};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use coordinate_systems::{Field, Ground, ImuReference, Local, Robot};
use linear_algebra::{Isometry2, Isometry3, Orientation2, Rotation2};

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct PoseEstimate<From, To> {
    pub pose: Isometry3<From, To, f64>,
    /// Right-local tangent covariance, ordered [rotation xyz, translation xyz].
    pub covariance: SMatrix<f64, 6, 6>,
}

impl<From, To> Message for PoseEstimate<From, To>
where
    From: Message + Serialize + DeserializeOwned,
    To: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!(
            "types::localization::PoseEstimate<{},{}>",
            From::type_name(),
            To::type_name()
        )
    }
}

impl<From, To> MessageSchema for PoseEstimate<From, To>
where
    From: Message + Serialize + DeserializeOwned,
    To: Message + Serialize + DeserializeOwned,
{
    fn build_schema(
        builder: &mut SchemaBuilder,
    ) -> Result<ros_z::__private::ros_z_schema::TypeDef, ros_z::__private::ros_z_schema::SchemaError>
    {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<Isometry3<From, To, f64>>("pose")?;
            let covariance = fields.shape::<[f64; 36]>()?;
            fields.field_with_shape("covariance", covariance);
            Ok(())
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message, PartialEq)]
pub struct LocalizationEstimate {
    pub time: Time,
    pub epoch: u64,
    /// Changes when bootstrap/recovery replaces Local within an epoch.
    pub generation: u64,
    pub robot_to_local: PoseEstimate<Robot, Local>,
    pub robot_to_field: Option<PoseEstimate<Robot, Field>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message, PartialEq, Eq)]
pub enum LocalizationState {
    Startup,
    Tracking,
    LostTrack,
}

/// An exposure-time robot heading constraint, independent of optimized Local.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message, PartialEq)]
pub struct HeadingConstraint {
    pub expected: Orientation2<Field, f64>,
    /// Absolute angular error in radians, strictly between zero and pi/2.
    pub max_error: f64,
}

impl HeadingConstraint {
    pub fn is_valid(&self) -> bool {
        (self.expected.inner.norm_sqr() - 1.0).abs() < 1e-6
            && self.max_error > 0.0
            && self.max_error < std::f64::consts::FRAC_PI_2
    }

    pub fn accepts(&self, heading: Orientation2<Field, f64>) -> bool {
        self.is_valid() && self.expected.rotation_to(heading).inner.angle().abs() <= self.max_error
    }
}

/// Read-only snapshot of localization's trusted IMU-to-field reference.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message, PartialEq)]
pub struct FieldHeadingReference {
    pub time: Time,
    pub imu_to_field: Rotation2<ImuReference, Field, f64>,
    pub max_error: f64,
}

impl FieldHeadingReference {
    pub fn at(
        &self,
        time: Time,
        imu_yaw: Orientation2<ImuReference, f64>,
    ) -> Option<HeadingConstraint> {
        let constraint = HeadingConstraint {
            expected: self.imu_to_field * imu_yaw,
            max_error: self.max_error,
        };
        (time >= self.time && constraint.is_valid()).then_some(constraint)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message, PartialEq)]
pub struct LocalizationStatus {
    pub time: Time,
    pub epoch: u64,
    pub generation: u64,
    pub state: LocalizationState,
    pub heading: Option<FieldHeadingReference>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub struct ScoredPose {
    pub state: MultivariateNormalDistribution<3>,
    pub score: f32,
}

pub fn ground_to_field_from_field_to_robot(
    field_to_robot: Isometry3<Field, Robot>,
    robot_to_ground: Isometry3<Robot, Ground>,
) -> Isometry2<Ground, Field> {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * robot_to_ground.inverse();
    let (_, _, yaw) = ground_to_field.inner.rotation.euler_angles();
    let translation = ground_to_field.inner.translation.vector;

    Isometry2::wrap(nalgebra::Isometry2::new(
        nalgebra::vector![translation.x, translation.y],
        yaw,
    ))
}

#[cfg(test)]
mod tests {
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn localization_output_schemas_are_valid() {
        LocalizationEstimate::schema();
        LocalizationStatus::schema();
    }

    #[test]
    fn ground_to_field_from_field_to_robot_flattens_robot_pose() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground = Isometry3::identity();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.translation().x() - 1.5).abs() < 1.0e-6);
        assert!((ground_to_field.translation().y() + 2.0).abs() < 1.0e-6);
        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn ground_to_field_cancels_consistent_body_tilt() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.2, 0.3, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground: Isometry3<Robot, Ground> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 0.0, 0.523),
            nalgebra::UnitQuaternion::from_euler_angles(0.2, 0.3, 0.0),
        )
        .framed_transform();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }
}
