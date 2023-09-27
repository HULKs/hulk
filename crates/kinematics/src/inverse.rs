use std::f32::consts::PI;

use nalgebra::{geometry::Isometry3, Rotation3, Translation3, Vector3};
use types::{joints::LegJoints, robot_dimensions::RobotDimensions};

pub fn leg_angles(
    left_foot_to_torso: Isometry3<f32>,
    right_foot_to_torso: Isometry3<f32>,
) -> (bool, LegJoints<f32>, LegJoints<f32>) {
    let ratio = 0.5;
    let torso_to_left_pelvis = Isometry3::rotation(Vector3::x() * -1.0 * PI / 4.0)
        * Translation3::from(-RobotDimensions::ROBOT_TO_LEFT_PELVIS);
    let torso_to_right_pelvis = Isometry3::rotation(Vector3::x() * PI / 4.0)
        * Translation3::from(-RobotDimensions::ROBOT_TO_RIGHT_PELVIS);

    let left_foot_to_left_pelvis = torso_to_left_pelvis * left_foot_to_torso;
    let right_foot_to_right_pelvis = torso_to_right_pelvis * right_foot_to_torso;
    let vector_left_foot_to_left_pelvis = left_foot_to_left_pelvis.inverse().translation;
    let vector_right_foot_to_right_pelvis = right_foot_to_right_pelvis.inverse().translation;

    let left_foot_roll_in_pelvis = vector_left_foot_to_left_pelvis
        .y
        .atan2(vector_left_foot_to_left_pelvis.z);
    let right_foot_roll_in_pelvis = vector_right_foot_to_right_pelvis
        .y
        .atan2(vector_right_foot_to_right_pelvis.z);

    let left_foot_pitch_2_in_pelvis = vector_left_foot_to_left_pelvis.x.atan2(
        (vector_left_foot_to_left_pelvis.y.powi(2) + vector_left_foot_to_left_pelvis.z.powi(2))
            .sqrt(),
    );
    let right_foot_pitch_2_in_pelvis = vector_right_foot_to_right_pelvis.x.atan2(
        (vector_right_foot_to_right_pelvis.y.powi(2) + vector_right_foot_to_right_pelvis.z.powi(2))
            .sqrt(),
    );

    let left_hip_rotation_c1 = left_foot_to_left_pelvis.rotation
        * ((Rotation3::new(Vector3::x() * -1.0 * left_foot_roll_in_pelvis)
            * Rotation3::new(Vector3::y() * left_foot_pitch_2_in_pelvis))
            * Vector3::y());
    let right_hip_rotation_c1 = right_foot_to_right_pelvis.rotation
        * ((Rotation3::new(Vector3::x() * -1.0 * right_foot_roll_in_pelvis)
            * Rotation3::new(Vector3::y() * right_foot_pitch_2_in_pelvis))
            * Vector3::y());

    let left_hip_yaw_pitch = -1.0 * (-1.0 * left_hip_rotation_c1.x).atan2(left_hip_rotation_c1.y);
    let right_hip_yaw_pitch = (-1.0 * right_hip_rotation_c1.x).atan2(right_hip_rotation_c1.y);
    let left_hip_yaw_pitch_combined =
        left_hip_yaw_pitch * ratio + right_hip_yaw_pitch * (1.0 - ratio);

    let left_pelvis_to_left_hip = Isometry3::rotation(Vector3::z() * left_hip_yaw_pitch_combined);
    let left_foot_to_left_hip = left_pelvis_to_left_hip * left_foot_to_left_pelvis;
    let right_pelvis_to_right_hip =
        Isometry3::rotation(Vector3::z() * -1.0 * left_hip_yaw_pitch_combined);
    let right_foot_to_right_hip = right_pelvis_to_right_hip * right_foot_to_right_pelvis;

    let vector_left_hip_to_left_foot = left_foot_to_left_hip.translation;
    let vector_right_hip_to_right_foot = right_foot_to_right_hip.translation;

    let left_hip_roll_in_hip =
        -1.0 * (-1.0 * vector_left_hip_to_left_foot.y).atan2(-1.0 * vector_left_hip_to_left_foot.z);
    let right_hip_roll_in_hip = -1.0
        * (-1.0 * vector_right_hip_to_right_foot.y).atan2(-1.0 * vector_right_hip_to_right_foot.z);

    let left_hip_pitch_minus_alpha = (-1.0 * vector_left_hip_to_left_foot.x).atan2(
        (vector_left_hip_to_left_foot.y.powi(2) + vector_left_hip_to_left_foot.z.powi(2)).sqrt()
            * -1.0
            * vector_left_hip_to_left_foot.z.signum(),
    );
    let right_hip_pitch_minus_alpha = (-1.0 * vector_right_hip_to_right_foot.x).atan2(
        (vector_right_hip_to_right_foot.y.powi(2) + vector_right_hip_to_right_foot.z.powi(2))
            .sqrt()
            * -1.0
            * vector_right_hip_to_right_foot.z.signum(),
    );

    let left_foot_rotation_c2 =
        Isometry3::rotation(Vector3::y() * -1.0 * left_hip_pitch_minus_alpha)
            * Isometry3::rotation(Vector3::x() * -1.0 * left_hip_roll_in_hip)
            * (left_foot_to_left_hip.rotation * Vector3::z());
    let right_foot_rotation_c2 =
        Isometry3::rotation(Vector3::y() * -1.0 * right_hip_pitch_minus_alpha)
            * Isometry3::rotation(Vector3::x() * -1.0 * right_hip_roll_in_hip)
            * (right_foot_to_right_hip.rotation * Vector3::z());

    let upper_leg = RobotDimensions::HIP_TO_KNEE.z.abs();
    let lower_leg = RobotDimensions::KNEE_TO_ANKLE.z.abs();
    let left_height = left_foot_to_left_hip.translation.vector.norm();
    let right_height = right_foot_to_right_hip.translation.vector.norm();

    let left_cos_minus_alpha = (upper_leg.powi(2) + left_height.powi(2) - lower_leg.powi(2))
        / (2.0 * upper_leg * left_height);
    let right_cos_minus_alpha = (upper_leg.powi(2) + right_height.powi(2) - lower_leg.powi(2))
        / (2.0 * upper_leg * right_height);
    let left_cos_minus_beta = (lower_leg.powi(2) + left_height.powi(2) - upper_leg.powi(2))
        / (2.0 * lower_leg * left_height);
    let right_cos_minus_beta = (lower_leg.powi(2) + right_height.powi(2) - upper_leg.powi(2))
        / (2.0 * lower_leg * right_height);
    let left_alpha = -1.0 * left_cos_minus_alpha.clamp(-1.0, 1.0).acos();
    let right_alpha = -1.0 * right_cos_minus_alpha.clamp(-1.0, 1.0).acos();
    let left_beta = -1.0 * left_cos_minus_beta.clamp(-1.0, 1.0).acos();
    let right_beta = -1.0 * right_cos_minus_beta.clamp(-1.0, 1.0).acos();

    let left_leg = LegJoints {
        hip_yaw_pitch: left_hip_yaw_pitch_combined,
        hip_roll: left_hip_roll_in_hip + PI / 4.0,
        hip_pitch: left_hip_pitch_minus_alpha + left_alpha,
        knee_pitch: -left_alpha - left_beta,
        ankle_pitch: left_foot_rotation_c2.x.atan2(left_foot_rotation_c2.z) + left_beta,
        ankle_roll: (-1.0 * left_foot_rotation_c2.y).asin(),
    };
    let right_leg = LegJoints {
        hip_yaw_pitch: left_hip_yaw_pitch_combined,
        hip_roll: right_hip_roll_in_hip - PI / 4.0,
        hip_pitch: right_hip_pitch_minus_alpha + right_alpha,
        knee_pitch: -right_alpha - right_beta,
        ankle_pitch: right_foot_rotation_c2.x.atan2(right_foot_rotation_c2.z) + right_beta,
        ankle_roll: (-1.0 * right_foot_rotation_c2.y).asin(),
    };
    let maximum_leg_extension = upper_leg + lower_leg;
    let is_reachable =
        left_height <= maximum_leg_extension && right_height <= maximum_leg_extension;

    (is_reachable, left_leg, right_leg)
}
