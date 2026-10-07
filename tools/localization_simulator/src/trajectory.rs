use coordinate_systems::{Camera, Field, Robot};
use linear_algebra::{IntoTransform, Isometry3 as FramedIsometry3};
use nalgebra::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

const MAX_SCENARIO_DURATION_SECONDS: f32 = 600.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One camera-to-field trajectory keyframe.
pub struct PoseKeyframe {
    time_seconds: f32,
    position: [f32; 3],
    /// Quaternion components in x, y, z, w order.
    quaternion_xyzw: [f32; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
/// A validated, bounded six-degree-of-freedom camera trajectory.
pub struct Scenario {
    name: String,
    duration_seconds: f32,
    camera_to_field_keyframes: Vec<PoseKeyframe>,
    #[serde(skip)]
    tick_count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioDto {
    name: String,
    duration_seconds: f32,
    camera_to_field_keyframes: Vec<PoseKeyframe>,
}

impl<'de> Deserialize<'de> for Scenario {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dto = ScenarioDto::deserialize(deserializer)?;
        Self::new(
            dto.name,
            dto.duration_seconds,
            dto.camera_to_field_keyframes,
        )
        .map_err(D::Error::custom)
    }
}

impl Scenario {
    /// Constructs and validates a scenario.
    pub fn new(
        name: impl Into<String>,
        duration_seconds: f32,
        camera_to_field_keyframes: Vec<PoseKeyframe>,
    ) -> Result<Self, String> {
        let scenario = Self {
            name: name.into(),
            duration_seconds,
            camera_to_field_keyframes,
            tick_count: 0,
        };
        let tick_count = scenario.validate()?;
        let scenario = Self {
            tick_count,
            ..scenario
        };
        Ok(scenario)
    }

    fn validate(&self) -> Result<usize, String> {
        if self.name.trim().is_empty() {
            return Err("scenario name must not be empty".to_string());
        }
        if !self.duration_seconds.is_finite() || self.duration_seconds <= 0.0 {
            return Err("scenario duration must be finite and > 0".to_string());
        }
        if self.duration_seconds > MAX_SCENARIO_DURATION_SECONDS {
            return Err(format!(
                "scenario duration must not exceed {MAX_SCENARIO_DURATION_SECONDS} seconds"
            ));
        }
        let tick_count =
            (self.duration_seconds / crate::config::TICK_INTERVAL.as_secs_f32()).round() as usize;
        let aligned_duration = tick_count as f32 * crate::config::TICK_INTERVAL.as_secs_f32();
        if aligned_duration != self.duration_seconds {
            return Err(
                "scenario duration must be aligned to the 20 ms simulation tick".to_string(),
            );
        }
        if self.camera_to_field_keyframes.len() < 2 {
            return Err("scenario must contain at least two keyframes".to_string());
        }
        for (index, keyframe) in self.camera_to_field_keyframes.iter().enumerate() {
            if !keyframe.time_seconds.is_finite()
                || keyframe
                    .position
                    .into_iter()
                    .any(|value| !value.is_finite())
                || keyframe
                    .quaternion_xyzw
                    .into_iter()
                    .any(|value| !value.is_finite())
            {
                return Err(format!("keyframe {index} contains a non-finite value"));
            }
            let norm_squared = keyframe
                .quaternion_xyzw
                .into_iter()
                .map(|value| value * value)
                .sum::<f32>();
            if (norm_squared - 1.0).abs() > 1.0e-3 {
                return Err(format!("keyframe {index} quaternion must have unit length"));
            }
            if index > 0
                && keyframe.time_seconds <= self.camera_to_field_keyframes[index - 1].time_seconds
            {
                return Err("keyframe timestamps must be strictly increasing".to_string());
            }
        }
        let first = self.camera_to_field_keyframes[0].time_seconds;
        let last = self.camera_to_field_keyframes.last().unwrap().time_seconds;
        if first != 0.0 || (last - self.duration_seconds).abs() > 1.0e-6 {
            return Err("keyframes must span exactly from zero to scenario duration".to_string());
        }
        Ok(tick_count)
    }

    /// Returns the display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the duration in seconds.
    pub fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }

    /// Returns the validated keyframes.
    pub fn keyframes(&self) -> &[PoseKeyframe] {
        &self.camera_to_field_keyframes
    }

    /// Returns the last fixed simulation-tick index.
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    /// Samples the camera-to-field truth transform, clamping to the scenario endpoints.
    pub fn sample_camera_to_field(&self, time_seconds: f32) -> FramedIsometry3<Camera, Field> {
        let time_seconds = time_seconds.clamp(0.0, self.duration_seconds);
        let upper = self
            .camera_to_field_keyframes
            .partition_point(|keyframe| keyframe.time_seconds <= time_seconds);
        if upper == 0 {
            return keyframe_pose(&self.camera_to_field_keyframes[0]);
        }
        if upper == self.camera_to_field_keyframes.len() {
            return keyframe_pose(self.camera_to_field_keyframes.last().unwrap());
        }
        let start = &self.camera_to_field_keyframes[upper - 1];
        let end = &self.camera_to_field_keyframes[upper];
        let alpha = (time_seconds - start.time_seconds) / (end.time_seconds - start.time_seconds);
        keyframe_pose(start)
            .inner
            .lerp_slerp(&keyframe_pose(end).inner, alpha)
            .framed_transform()
    }

    /// Returns a stationary three-second view of nearby own-half markings for bootstrap.
    pub fn stationary() -> Self {
        from_robot_poses(
            "stationary",
            3.0,
            vec![
                (
                    0.0,
                    robot_pose(
                        [-3.5, 1.5, 0.55],
                        [0.0, 0.2, -5.0 * std::f32::consts::PI / 6.0],
                    ),
                ),
                (
                    3.0,
                    robot_pose(
                        [-3.5, 1.5, 0.55],
                        [0.0, 0.2, -5.0 * std::f32::consts::PI / 6.0],
                    ),
                ),
            ],
        )
    }

    /// Returns a smooth loop containing translation, roll, pitch, yaw, and height changes.
    pub fn six_dof_loop() -> Self {
        from_robot_poses(
            "six_dof_loop",
            4.0,
            vec![
                (0.0, robot_pose([-3.0, 0.0, 0.55], [0.0, 0.0, 0.0])),
                (1.0, robot_pose([-2.4, 0.5, 0.62], [0.12, -0.08, 0.35])),
                (2.0, robot_pose([-1.9, 0.0, 0.50], [-0.1, 0.1, 0.0])),
                (3.0, robot_pose([-2.4, -0.5, 0.58], [0.06, -0.12, -0.35])),
                (4.0, robot_pose([-3.0, 0.0, 0.55], [0.0, 0.0, 0.0])),
            ],
        )
    }

    /// Returns a trajectory containing a discontinuous ground-truth robot relocation.
    pub fn pose_teleport() -> Self {
        from_robot_poses(
            "pose_teleport",
            3.0,
            vec![
                (0.0, robot_pose([-3.0, 0.0, 0.55], [0.0, 0.0, 0.0])),
                (1.0, robot_pose([-2.8, 0.0, 0.55], [0.0, 0.0, 0.0])),
                (1.02, robot_pose([-1.8, 0.7, 0.65], [0.1, -0.15, 0.55])),
                (3.0, robot_pose([-1.4, 0.7, 0.65], [0.1, -0.15, 0.55])),
            ],
        )
    }

    /// Returns the smooth truth trajectory used with a synthetic VO-only fault.
    pub fn vo_fault() -> Self {
        from_robot_poses(
            "vo_fault",
            3.0,
            vec![
                (0.0, robot_pose([-3.0, 0.0, 0.55], [0.0, 0.0, 0.0])),
                (3.0, robot_pose([-2.4, 0.0, 0.55], [0.0, 0.0, 0.0])),
            ],
        )
    }

    /// Returns two laps of a field-spanning figure-eight with yaw aligned to the path tangent.
    pub fn field_figure_eight_twice() -> Self {
        const DURATION_SECONDS: f32 = 24.0;
        const SEGMENTS: usize = 80;
        // Acquire nearby markings on our half before traversing the symmetry boundary.
        const START_THETA: f32 = -std::f32::consts::FRAC_PI_2;
        let poses = (0..=SEGMENTS)
            .map(|index| {
                let progress = index as f32 / SEGMENTS as f32;
                let theta = progress * 4.0 * std::f32::consts::PI + START_THETA;
                let x = 4.0 * theta.sin();
                let y = 2.5 * (2.0 * theta).sin();
                let dx = 4.0 * theta.cos();
                let dy = 5.0 * (2.0 * theta).cos();
                let yaw = dy.atan2(dx);
                (
                    progress * DURATION_SECONDS,
                    robot_pose([x, y, 0.55], [0.0, 0.0, yaw]),
                )
            })
            .collect();
        from_robot_poses("field_figure_eight_twice", DURATION_SECONDS, poses)
    }
}

impl PoseKeyframe {
    /// Constructs a keyframe from a camera-to-field transform.
    pub fn from_camera_to_field(time_seconds: f32, pose: FramedIsometry3<Camera, Field>) -> Self {
        let quaternion = pose.inner.rotation.quaternion();
        Self {
            time_seconds,
            position: pose.inner.translation.vector.into(),
            quaternion_xyzw: [quaternion.i, quaternion.j, quaternion.k, quaternion.w],
        }
    }

    /// Returns the keyframe timestamp in seconds.
    pub fn time_seconds(&self) -> f32 {
        self.time_seconds
    }

    /// Reconstructs the camera-to-field transform.
    pub fn camera_to_field(&self) -> FramedIsometry3<Camera, Field> {
        keyframe_pose(self)
    }
}

/// Rotation mapping robot x/y/z to camera z/-x/-y respectively.
pub fn fixed_robot_to_camera() -> FramedIsometry3<Robot, Camera> {
    let rotation = nalgebra::Matrix3::new(0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0);
    Isometry3::from_parts(
        Translation3::identity(),
        UnitQuaternion::from_rotation_matrix(&nalgebra::Rotation3::from_matrix_unchecked(rotation)),
    )
    .framed_transform()
}

/// Converts a camera-to-field pose using the simulator's fixed robot-to-camera extrinsic.
pub fn robot_to_field_from_camera_to_field(
    camera_to_field: &FramedIsometry3<Camera, Field>,
) -> FramedIsometry3<Robot, Field> {
    camera_to_field * fixed_robot_to_camera()
}

fn robot_pose(position: [f32; 3], rpy: [f32; 3]) -> FramedIsometry3<Robot, Field> {
    Isometry3::from_parts(
        Translation3::from(Vector3::from(position)),
        UnitQuaternion::from_euler_angles(rpy[0], rpy[1], rpy[2]),
    )
    .framed_transform()
}

fn from_robot_poses(
    name: &str,
    duration_seconds: f32,
    poses: Vec<(f32, FramedIsometry3<Robot, Field>)>,
) -> Scenario {
    let camera_alignment_inverse = fixed_robot_to_camera().inverse();
    Scenario::new(
        name,
        duration_seconds,
        poses
            .into_iter()
            .map(|(time_seconds, robot_to_field)| {
                PoseKeyframe::from_camera_to_field(
                    time_seconds,
                    robot_to_field * camera_alignment_inverse,
                )
            })
            .collect(),
    )
    .expect("built-in trajectory is valid")
}

fn keyframe_pose(keyframe: &PoseKeyframe) -> FramedIsometry3<Camera, Field> {
    let [x, y, z, w] = keyframe.quaternion_xyzw;
    Isometry3::from_parts(
        Translation3::from(Vector3::from(keyframe.position)),
        UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z)),
    )
    .framed_transform()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_is_linear_and_uses_slerp() {
        let scenario = from_robot_poses(
            "interpolation",
            2.0,
            vec![
                (0.0, robot_pose([0.0, 0.0, 0.5], [0.0, 0.0, 0.0])),
                (
                    2.0,
                    robot_pose([2.0, 0.0, 0.5], [0.0, 0.0, std::f32::consts::FRAC_PI_2]),
                ),
            ],
        );
        let robot =
            robot_to_field_from_camera_to_field(&scenario.sample_camera_to_field(1.0)).inner;
        assert!((robot.translation.vector - Vector3::new(1.0, 0.0, 0.5)).norm() < 1.0e-5);
        assert!((robot.rotation.euler_angles().2 - std::f32::consts::FRAC_PI_4).abs() < 1.0e-5);
    }

    #[test]
    fn validation_rejects_bad_timeline_and_quaternion() {
        let invalid_timeline = r#"{
            name: "invalid",
            duration_seconds: 1.0,
            camera_to_field_keyframes: [
                { time_seconds: 0.0, position: [0, 0, 0], quaternion_xyzw: [0, 0, 0, 1] },
                { time_seconds: 0.0, position: [0, 0, 0], quaternion_xyzw: [0, 0, 0, 1] },
            ],
        }"#;
        assert!(json5::from_str::<Scenario>(invalid_timeline).is_err());

        let invalid_quaternion = r#"{
            name: "invalid",
            duration_seconds: 1.0,
            camera_to_field_keyframes: [
                { time_seconds: 0.0, position: [0, 0, 0], quaternion_xyzw: [0, 0, 0, 0] },
                { time_seconds: 1.0, position: [0, 0, 0], quaternion_xyzw: [0, 0, 0, 1] },
            ],
        }"#;
        assert!(json5::from_str::<Scenario>(invalid_quaternion).is_err());
    }

    #[test]
    fn fixed_alignment_maps_robot_axes_to_camera_convention() {
        let alignment = fixed_robot_to_camera().inner;
        assert!((alignment * Vector3::x() - Vector3::z()).norm() < 1.0e-6);
        assert!((alignment * Vector3::y() + Vector3::x()).norm() < 1.0e-6);
        assert!((alignment * Vector3::z() + Vector3::y()).norm() < 1.0e-6);
    }

    #[test]
    fn scenario_duration_is_bounded() {
        let pose = PoseKeyframe::from_camera_to_field(0.0, FramedIsometry3::identity());
        let end = PoseKeyframe::from_camera_to_field(
            MAX_SCENARIO_DURATION_SECONDS + 0.02,
            FramedIsometry3::identity(),
        );

        assert!(
            Scenario::new(
                "too_long",
                MAX_SCENARIO_DURATION_SECONDS + 0.02,
                vec![pose, end]
            )
            .is_err()
        );
    }

    #[test]
    fn teleport_and_vo_fault_have_distinct_truth_motion() {
        let teleport = Scenario::pose_teleport();
        let before = teleport.sample_camera_to_field(1.0);
        let after = teleport.sample_camera_to_field(1.02);
        assert!((after.translation() - before.translation()).norm() > 0.5);

        let vo_fault = Scenario::vo_fault();
        let before = vo_fault.sample_camera_to_field(1.0);
        let after = vo_fault.sample_camera_to_field(1.02);
        assert!((after.translation() - before.translation()).norm() < 0.1);
    }

    #[test]
    fn field_figure_eight_spans_the_field_and_repeats_twice() {
        let scenario = Scenario::field_figure_eight_twice();
        let samples = (0..=1_200)
            .map(|tick| {
                robot_to_field_from_camera_to_field(
                    &scenario.sample_camera_to_field(tick as f32 * 0.02),
                )
            })
            .collect::<Vec<_>>();
        let max_x = samples
            .iter()
            .map(|pose| pose.translation().x().abs())
            .fold(0.0_f32, f32::max);
        let max_y = samples
            .iter()
            .map(|pose| pose.translation().y().abs())
            .fold(0.0_f32, f32::max);
        let start = &samples[0];
        let after_one_lap = &samples[600];
        let after_two_laps = &samples[1_200];

        assert!(max_x > 3.9);
        assert!(max_y > 2.4);
        assert!(start.translation().x() < 0.0);
        assert!(start.translation().x() < -3.9);
        assert!(start.translation().y().abs() < 1.0e-5);
        assert!((start.translation() - after_one_lap.translation()).norm() < 1.0e-5);
        assert!((start.translation() - after_two_laps.translation()).norm() < 1.0e-5);
        assert!(start.inner.rotation.angle_to(&after_one_lap.inner.rotation) < 1.0e-5);
        assert!(
            start
                .inner
                .rotation
                .angle_to(&after_two_laps.inner.rotation)
                < 1.0e-5
        );
    }

    #[test]
    fn built_in_scenarios_start_on_own_half() {
        for scenario in [
            Scenario::stationary(),
            Scenario::six_dof_loop(),
            Scenario::field_figure_eight_twice(),
            Scenario::pose_teleport(),
            Scenario::vo_fault(),
        ] {
            let robot_to_field =
                robot_to_field_from_camera_to_field(&scenario.sample_camera_to_field(0.0));
            assert!(
                robot_to_field.translation().x() < -0.05,
                "{} starts outside its own half",
                scenario.name()
            );
        }
    }
}
