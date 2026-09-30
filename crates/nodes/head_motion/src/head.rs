//! Head target selection and joint control, independent of ROS interfaces and logging.

use std::{sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use coordinate_systems::Ground;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Rotation2, point};
use ros_z::time::Time;
use types::{
    field_dimensions::GlobalFieldSide,
    joint_limits::JointLimits,
    motion_command::{HeadMotion, ImageRegion},
    motor_command::MotorCommand,
    support_foot::Side,
};

use crate::{
    joint_control::{HeadObservation, JointTarget},
    look_at::{LookAtError, LookAtGeometry, LookAtTarget, look_at},
    parameters::Parameters,
    patterns::{GlanceState, ScanKind, ScanState},
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
    Scan(ScanKind),
    Glance,
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
    glance: GlanceState,
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
            self.glance = GlanceState::default();
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
        let (look_at_target, travel_speed) = match *request {
            HeadMotion::ZeroAngles => {
                return HeadTarget::new(move_to(
                    HeadJoints::fill(0.0),
                    parameters.direct_travel_speed,
                ));
            }
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
                (
                    LookAtTarget {
                        position: point![width / 2.0, 0.0, 0.0],
                        image_region: image_region_target,
                    },
                    parameters.direct_travel_speed,
                )
            }
            HeadMotion::LookAt {
                target,
                height_above_ground,
                image_region_target,
            } => (
                LookAtTarget {
                    position: point![target.x(), target.y(), height_above_ground],
                    image_region: image_region_target,
                },
                parameters.direct_travel_speed,
            ),
            HeadMotion::GlanceLeftAndRightOf {
                target,
                height_above_ground,
            } => {
                let angle = self.glance.angle(
                    parameters.glance.angle,
                    parameters.glance.phase_duration,
                    now,
                );
                let target = Rotation2::<Ground, Ground>::new(angle) * target;
                (
                    LookAtTarget {
                        position: point![target.x(), target.y(), height_above_ground],
                        image_region: ImageRegion::Center,
                    },
                    parameters.glance.travel_speed,
                )
            }
        };
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
        self.glance = GlanceState::default();
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
        HeadMotion::GlanceLeftAndRightOf { .. } => Mode::Glance,
        HeadMotion::Damping => Mode::Damping,
        HeadMotion::ZeroAngles | HeadMotion::Center { .. } | HeadMotion::LookAt { .. } => {
            Mode::Direct
        }
        HeadMotion::MoveWithVelocity { .. } => Mode::Injected,
    }
}
