//! Head target selection and joint control, independent of ROS interfaces and logging.

use std::{sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use kinematics::joints::head::HeadJoints;
use linear_algebra::point;
use ros_z::time::Time;
use types::{
    field_dimensions::GlobalFieldSide, joint_limits::JointLimits, motion_command::HeadMotion,
    motor_command::MotorCommand, support_foot::Side,
};

use crate::{
    joint_control::{HeadObservation, JointTarget},
    look_at::{LookAtError, LookAtGeometry, LookAtTarget, look_at},
    parameters::Parameters,
    patterns::{ScanKind, ScanState},
};

pub(crate) struct HeadInputs {
    pub(crate) parameters: Arc<Parameters>,
    pub(crate) joint_limits: Arc<JointLimits>,
    pub(crate) geometry: Option<LookAtGeometry>,
    pub(crate) field_width: Option<f32>,
    pub(crate) global_field_side: Option<GlobalFieldSide>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoldReason {
    MissingGeometry,
    MissingFieldDimensions,
    InvalidFieldWidth,
    Geometry(LookAtError),
}

pub(crate) struct HeadOutput {
    pub(crate) commands: HeadJoints<MotorCommand>,
    pub(crate) hold_reason: Option<HoldReason>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Direct,
    Velocity,
    Scan(ScanKind),
    Damping,
    Injected,
}

struct TimedObservation {
    value: HeadObservation,
    time: Time,
}

#[derive(Default)]
pub(crate) struct HeadController {
    observation: Option<TimedObservation>,
    mode: Option<Mode>,
    last_evaluation: Option<Time>,
    last_commanded_position: Option<HeadJoints<f32>>,
    hold_position: Option<HeadJoints<f32>>,
    scan: ScanState,
}

impl HeadController {
    pub(crate) fn observe(
        &mut self,
        observation: HeadObservation,
        source_time: Time,
    ) -> Result<()> {
        observation.validate()?;
        if self
            .observation
            .as_ref()
            .is_some_and(|previous| source_time < previous.time)
        {
            self.reset_motion();
        }
        self.observation = Some(TimedObservation {
            value: observation,
            time: source_time,
        });
        Ok(())
    }

    pub(crate) fn evaluate(
        &mut self,
        request: &HeadMotion,
        inputs: &HeadInputs,
        now: Time,
    ) -> Result<HeadOutput> {
        let parameters = &inputs.parameters;
        let observation = self.current_observation(now, parameters.maximum_observation_age)?;
        let joint_limits = &inputs.joint_limits;

        let mode = mode_for(request, parameters.injected_head_joints.is_some());
        self.prepare_mode(mode, now, parameters.joint_control.reseed_after);
        let (start_position, elapsed) = match self.last_commanded_position.zip(self.last_evaluation)
        {
            Some((position, time)) => (position, now.duration_since(time).as_secs_f32()),
            None => (observation.positions, 0.0),
        };
        let HeadTarget {
            target,
            hold_reason,
        } = self.compute_target(request, inputs, start_position, now);
        if hold_reason.is_none() {
            self.hold_position = None;
        }
        let commands = target.motor_commands(
            start_position,
            elapsed,
            &parameters.joint_control,
            joint_limits,
        );
        self.last_commanded_position = if mode == Mode::Damping {
            None
        } else {
            Some(HeadJoints {
                yaw: commands.yaw.position,
                pitch: commands.pitch.position,
            })
        };
        self.last_evaluation = Some(now);
        Ok(HeadOutput {
            commands,
            hold_reason,
        })
    }

    fn current_observation(&self, now: Time, maximum_age: Duration) -> Result<HeadObservation> {
        let observation = self
            .observation
            .as_ref()
            .ok_or_else(|| eyre!("head observation is unavailable"))?;
        ensure!(
            now >= observation.time,
            "head observation source time {:?} is ahead of node time {now:?}; \
             check source/node clock alignment or clock rollback",
            observation.time,
        );
        let age = now.duration_since(observation.time);
        ensure!(
            age <= maximum_age,
            "latest valid head observation is stale: age={age:?}, maximum_age={maximum_age:?}; \
             check motor-state delivery and source/node clock alignment"
        );
        Ok(observation.value)
    }

    fn prepare_mode(&mut self, mode: Mode, now: Time, reseed_after: Duration) {
        if self
            .last_evaluation
            .is_some_and(|last| now < last || now.duration_since(last) > reseed_after)
        {
            self.reset_motion();
        }
        if self.mode != Some(mode) {
            self.scan = ScanState::default();
            self.hold_position = None;
        }
        self.mode = Some(mode);
    }

    fn compute_target(
        &mut self,
        request: &HeadMotion,
        inputs: &HeadInputs,
        start_position: HeadJoints<f32>,
        now: Time,
    ) -> HeadTarget {
        let parameters = &inputs.parameters;
        if let Some(position) = parameters.injected_head_joints {
            return HeadTarget::new(move_to(position, parameters.direct_travel_speed));
        }
        let look_at_target = match *request {
            HeadMotion::Damping => return HeadTarget::new(JointTarget::Damping),
            HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                let kind = if matches!(request, HeadMotion::LookAround) {
                    ScanKind::LookAround
                } else {
                    ScanKind::SearchForLostBall
                };
                let side = if inputs.global_field_side == Some(GlobalFieldSide::Away) {
                    Side::Right
                } else {
                    Side::Left
                };
                let scan_parameters = match kind {
                    ScanKind::LookAround => &parameters.look_around,
                    ScanKind::SearchForLostBall => &parameters.search_for_lost_ball,
                };
                let target_position = self.scan.update(kind, side, scan_parameters, now);
                return HeadTarget::new(move_to(target_position, scan_parameters.travel_speed));
            }
            HeadMotion::MoveWithVelocity { yaw, pitch } => {
                return HeadTarget::new(JointTarget::MoveWithVelocity {
                    velocity: HeadJoints { yaw, pitch },
                });
            }
            HeadMotion::Center {
                image_region_target,
            } => {
                let Some(width) = inputs.field_width else {
                    return self.hold(
                        HoldReason::MissingFieldDimensions,
                        start_position,
                        parameters.direct_travel_speed,
                    );
                };
                if !width.is_finite() || width <= 0.0 {
                    return self.hold(
                        HoldReason::InvalidFieldWidth,
                        start_position,
                        parameters.direct_travel_speed,
                    );
                }
                LookAtTarget {
                    position: point![width / 2.0, 0.0, 0.0],
                    image_region: image_region_target,
                }
            }
            HeadMotion::LookAt {
                target,
                height_above_ground,
                image_region_target,
            } => LookAtTarget {
                position: point![target.x(), target.y(), height_above_ground],
                image_region: image_region_target,
            },
        };
        let travel_speed = parameters.direct_travel_speed;
        let Some(geometry) = inputs.geometry.as_ref() else {
            return self.hold(HoldReason::MissingGeometry, start_position, travel_speed);
        };
        match look_at(
            &look_at_target,
            geometry,
            &parameters.image_region_parameters,
            start_position,
        ) {
            Ok(target_position) => HeadTarget::new(move_to(target_position, travel_speed)),
            Err(error) => self.hold(HoldReason::Geometry(error), start_position, travel_speed),
        }
    }

    fn hold(
        &mut self,
        reason: HoldReason,
        start_position: HeadJoints<f32>,
        travel_speed: HeadJoints<f32>,
    ) -> HeadTarget {
        let hold_position = *self.hold_position.get_or_insert(start_position);
        HeadTarget {
            target: move_to(hold_position, travel_speed),
            hold_reason: Some(reason),
        }
    }

    /// Reseed position control from measurements while preserving pattern timing.
    pub(crate) fn clear_command_history(&mut self) {
        self.last_commanded_position = None;
        self.hold_position = None;
    }

    fn reset_motion(&mut self) {
        self.mode = None;
        self.last_evaluation = None;
        self.clear_command_history();
        self.scan = ScanState::default();
    }
}

struct HeadTarget {
    target: JointTarget,
    hold_reason: Option<HoldReason>,
}

impl HeadTarget {
    fn new(target: JointTarget) -> Self {
        Self {
            target,
            hold_reason: None,
        }
    }
}

fn move_to(position: HeadJoints<f32>, travel_speed: HeadJoints<f32>) -> JointTarget {
    JointTarget::MoveTo {
        position,
        travel_speed,
    }
}

fn mode_for(request: &HeadMotion, injected: bool) -> Mode {
    if injected {
        return Mode::Injected;
    }
    match request {
        HeadMotion::LookAround => Mode::Scan(ScanKind::LookAround),
        HeadMotion::SearchForLostBall => Mode::Scan(ScanKind::SearchForLostBall),
        HeadMotion::Damping => Mode::Damping,
        HeadMotion::Center { .. } | HeadMotion::LookAt { .. } => Mode::Direct,
        HeadMotion::MoveWithVelocity { .. } => Mode::Velocity,
    }
}

#[cfg(test)]
mod tests {
    //! Scenarios against the shipped parameters at the 500 Hz request rate of the motion node.
    //! The simulated head tracks every stiff command perfectly.

    use std::fs;

    use coordinate_systems::Ground;
    use kinematics::{
        forward::{head_to_camera, head_to_robot},
        joints::head::HeadJoint,
    };
    use linear_algebra::{Isometry3, Orientation3, Point3, Rotation3, vector};
    use projection::camera_matrix::CameraMatrix;
    use serde::{Deserialize, de::DeserializeOwned};
    use types::motion_command::ImageRegion;

    use super::*;

    const CYCLE: Duration = Duration::from_millis(2);

    fn load<T: DeserializeOwned>(file: &str) -> T {
        let path = format!(
            "{}/../../../etc/parameters/base/{file}",
            env!("CARGO_MANIFEST_DIR")
        );
        json5::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    fn inputs() -> HeadInputs {
        #[derive(Deserialize)]
        struct Global {
            joint_limits: JointLimits,
        }

        let parameters: Parameters = load("head_motion.json5");
        parameters.validate().unwrap();
        let ground_to_robot = Isometry3::from_parts(
            vector![0.0, 0.0, -0.55],
            Orientation3::from_euler_angles(0.02, 0.05, 0.0),
        );
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.55, 0.73],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
            ground_to_robot,
            head_to_robot(&HeadJoints::default()).inverse(),
            head_to_camera(0.0),
        )
        .to_corrected(
            Rotation3::from_euler_angles(0.01, -0.03, 0.02),
            Rotation3::from_euler_angles(-0.02, 0.01, 0.03),
        );
        HeadInputs {
            parameters: Arc::new(parameters),
            joint_limits: Arc::new(load::<Global>("global.json5").joint_limits),
            geometry: Some(LookAtGeometry {
                camera_matrix,
                ground_to_robot,
            }),
            field_width: Some(5.95),
            global_field_side: None,
        }
    }

    struct SimulatedHead {
        controller: HeadController,
        inputs: HeadInputs,
        now: Time,
        measured: HeadJoints<f32>,
    }

    impl SimulatedHead {
        fn new() -> Self {
            Self {
                controller: HeadController::default(),
                inputs: inputs(),
                now: Time::from_nanos(1_000_000_000),
                measured: HeadJoints::default(),
            }
        }

        fn run(&mut self, request: HeadMotion, duration: Duration) -> Vec<HeadOutput> {
            (0..duration.as_millis() / CYCLE.as_millis())
                .map(|_| {
                    self.now = self.now + CYCLE;
                    self.controller
                        .observe(
                            HeadObservation {
                                positions: self.measured,
                            },
                            self.now,
                        )
                        .unwrap();
                    let output = self
                        .controller
                        .evaluate(&request, &self.inputs, self.now)
                        .unwrap();
                    if output.commands.yaw.kp > 0.0 {
                        self.measured = positions(&output);
                    }
                    output
                })
                .collect()
        }

        fn pixel_error(
            &self,
            joints: HeadJoints<f32>,
            target: Point3<Ground>,
            region: ImageRegion,
        ) -> f32 {
            let geometry = self.inputs.geometry.as_ref().unwrap();
            let camera = &geometry.camera_matrix;
            let regions = &self.inputs.parameters.image_region_parameters;
            let normalized = match region {
                ImageRegion::Bottom => regions.bottom,
                ImageRegion::Center => regions.center,
                ImageRegion::Top => regions.top,
            };
            let requested = point![
                normalized.x() * camera.image_size.x(),
                normalized.y() * camera.image_size.y()
            ];
            let in_camera = camera.ground_to_camera_at(&joints, geometry.ground_to_robot) * target;
            assert!(in_camera.z() > 0.0, "target is behind the camera");
            (camera.intrinsics.project(in_camera.coords()) - requested).norm()
        }
    }

    fn positions(output: &HeadOutput) -> HeadJoints<f32> {
        HeadJoints {
            yaw: output.commands.yaw.position,
            pitch: output.commands.pitch.position,
        }
    }

    #[test]
    fn look_at_frames_targets_in_the_requested_image_region() {
        for (target, height_above_ground, image_region_target) in [
            (point![1.0, 0.3], 0.105, ImageRegion::Center),
            (point![2.5, -0.8], 0.0, ImageRegion::Center),
            (point![0.6, 0.0], 0.0, ImageRegion::Bottom),
            (point![4.0, 1.0], 0.0, ImageRegion::Top),
        ] {
            let mut head = SimulatedHead::new();
            let outputs = head.run(
                HeadMotion::LookAt {
                    target,
                    height_above_ground,
                    image_region_target,
                },
                Duration::from_secs(2),
            );

            assert!(outputs.iter().all(|output| output.hold_reason.is_none()));
            let target = point![target.x(), target.y(), height_above_ground];
            let error = head.pixel_error(
                positions(outputs.last().unwrap()),
                target,
                image_region_target,
            );
            assert!(
                error < 1.0,
                "{target:?} misses {image_region_target:?} by {error} px"
            );
        }
    }

    #[test]
    fn commands_respect_joint_and_speed_limits_across_requests() {
        let mut head = SimulatedHead::new();
        let limits = head.inputs.joint_limits.position.head;
        let maximum_step =
            head.inputs.parameters.joint_control.maximum_velocity * CYCLE.as_secs_f32();
        let outputs: Vec<_> = [
            (HeadMotion::LookAround, 5),
            (
                HeadMotion::LookAt {
                    target: point![0.5, 3.0],
                    height_above_ground: 0.0,
                    image_region_target: ImageRegion::Center,
                },
                1,
            ),
            (HeadMotion::Damping, 1),
            (
                HeadMotion::MoveWithVelocity {
                    yaw: -10.0,
                    pitch: 10.0,
                },
                1,
            ),
            (HeadMotion::SearchForLostBall, 5),
            (
                HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
                1,
            ),
        ]
        .into_iter()
        .flat_map(|(request, seconds)| head.run(request, Duration::from_secs(seconds)))
        .collect();

        for (previous, output) in outputs.iter().zip(&outputs[1..]) {
            assert!(output.hold_reason.is_none());
            if output.commands.yaw.kp == 0.0 {
                assert!(output.commands.yaw.kd > 0.0 && output.commands.pitch.kp == 0.0);
                continue;
            }
            for joint in [HeadJoint::Yaw, HeadJoint::Pitch] {
                let [minimum, maximum] = limits[joint];
                let position = output.commands[joint].position;
                let step = position - previous.commands[joint].position;
                assert!((minimum..=maximum).contains(&position));
                assert!(
                    step.abs() <= maximum_step[joint] + 1e-5,
                    "{joint:?} step {step}"
                );
            }
        }
    }
}
