use std::{f32::consts::PI, time::Duration};

use anyhow::Result;
use log::warn;
use macros::{module, require_some, SerializeHierarchy};
use nalgebra::{geometry::Isometry3, Point3, Vector3};
use serde::{Deserialize, Serialize};

use crate::{
    control::filtering::LowPassFilter,
    kinematics,
    types::{
        ArmJoints, BodyJoints, BodyMotionSafeExits, BodyMotionType, GroundContact, Joints,
        LegJoints, RobotDimensions, SensorData, Side, Step, SupportFoot, WalkCommand,
        WalkPositions,
    },
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum WalkState {
    Standing,
    Starting(Step),
    Walking(Step),
    Stopping,
}

impl Default for WalkState {
    fn default() -> Self {
        Self::Standing
    }
}

impl WalkState {
    fn next_walk_state(self, requested_walk_action: WalkCommand) -> Self {
        match (self, requested_walk_action) {
            (WalkState::Standing, WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Standing, WalkCommand::Walk(step)) => WalkState::Starting(step),
            (WalkState::Starting(_), WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Starting(_), WalkCommand::Walk(step)) => WalkState::Walking(step),
            (WalkState::Walking(_), WalkCommand::Stand) => WalkState::Stopping,
            (WalkState::Walking(_), WalkCommand::Walk(step)) => WalkState::Walking(step),
            (WalkState::Stopping, WalkCommand::Stand) => WalkState::Standing,
            (WalkState::Stopping, WalkCommand::Walk(step)) => WalkState::Walking(step),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
struct FootOffsets {
    forward: f32,
    left: f32,
}

/// # WalkingEngine
/// This module generates foot positions and thus leg angles for the robot to execute a walk.
/// The algorithm to compute the feet trajectories is loosely based on the work of Bernhard Hengst
/// at the team rUNSWift. An explanation of this alogrithm can be found in the team's research
/// report from 2014 (<http://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>).
#[derive(Default, Debug, Clone, SerializeHierarchy, Serialize, Deserialize)]
pub struct WalkingEngine {
    #[leaf]
    walk_state: WalkState,

    /// the step request from planning the engine is currently executing
    requested_step: Step,
    /// the lift (z-offset) the swing foot will have at its apex
    max_swing_foot_lift: f32,

    /// current planned offset of the left foot
    left_foot: FootOffsets,
    /// current planned offset of the left foot
    right_foot: FootOffsets,
    /// current planned turn component
    turn: f32,

    /// FootOffsets of the left foot when the support foot changed, t == 0 at the start of each
    /// walk phase
    left_foot_t0: FootOffsets,
    /// FootOffsets of the right foot when the support foot changed, t == 0 at the start of each
    /// walk phase
    right_foot_t0: FootOffsets,
    /// turn component when the support foot changed, t == 0 at the start of each walk phase
    turn_t0: f32,

    /// current z-offset of the left foot
    left_foot_lift: f32,
    /// current z-offset of the right foot
    right_foot_lift: f32,

    /// foot lift (z-offset) of the swing foot at the end of the last walk phase
    max_foot_lift_last_step: f32,

    /// time (s) in the walk phase
    t: Duration,
    /// The relative time when the last phase ended
    t_on_last_phase_end: Duration,
    /// The duration the currently executed step is planned to take
    step_duration: Duration,
    /// Fix the side of the swing foot for an entire walk phase
    #[leaf]
    swing_side: Side,
    #[leaf]
    /// Low pass filter the gyro for balance adjustment
    filtered_gyro_y: LowPassFilter<f32>,
    #[leaf]
    /// Low pass filter the torso for step adjustments
    filtered_torso_offset: LowPassFilter<f32>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = walk_command, data_type = WalkCommand)]
#[input(path = ground_contact, data_type = GroundContact)]
#[input(path = robot_to_ground, data_type = Isometry3<f32>)]
#[persistent_state(path = body_motion_safe_exits, data_type = BodyMotionSafeExits)]
#[parameter(path = control.walking_engine.walk_hip_height, data_type = f32)]
#[parameter(path = control.walking_engine.torso_offset, data_type = f32)]
#[parameter(path = control.walking_engine.minimal_step_duration, data_type = Duration)]
#[parameter(path = control.walking_engine.shoulder_pitch_factor, data_type = f32)]
#[parameter(path = control.walking_engine.base_foot_lift, data_type = f32)]
#[parameter(path = control.walking_engine.base_step_duration, data_type = Duration)]
#[parameter(path = control.walking_engine.step_duration_increase, data_type = Step)]
#[parameter(path = control.walking_engine.starting_step_duration, data_type = Duration)]
#[parameter(path = control.walking_engine.starting_step_foot_lift, data_type = f32)]
#[parameter(path = control.walking_engine.gyro_balance_factor, data_type = f32)]
#[parameter(path = control.walking_engine.swing_foot_leveling_factor, data_type = f32)]
#[parameter(path = control.walking_engine.max_forward_acceleration, data_type = f32)]
#[parameter(path = control.walking_engine.forward_foot_support_offset, data_type = f32)]
#[parameter(path = control.walking_engine.backward_foot_support_offset, data_type = f32)]
#[parameter(path = control.walking_engine.max_step_adjustment, data_type = f32)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[additional_output(path = walking_engine, data_type = WalkingEngine)]
#[main_output(data_type = WalkPositions)]
impl WalkingEngine {}

impl WalkingEngine {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            filtered_gyro_y: LowPassFilter::with_alpha(0.0, 0.1),
            filtered_torso_offset: LowPassFilter::with_alpha(0.0, 0.1),
            ..Default::default()
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let ground_contact = require_some!(context.ground_contact);
        let support_foot = require_some!(context.support_foot);
        let walk_command = require_some!(context.walk_command);

        self.filtered_gyro_y
            .update(sensor_data.inertial_measurement_unit.angular_velocity.y);
        if let Some(robot_to_ground) = *context.robot_to_ground {
            let torso_projected_onto_ground =
                (robot_to_ground * Point3::origin()).x - *context.torso_offset;
            self.filtered_torso_offset
                .update(torso_projected_onto_ground);
        } else {
            self.filtered_torso_offset.reset(0.0);
        }

        if self.t.is_zero() {
            self.initialize_step_states_from_request(
                *walk_command,
                support_foot.support_side,
                *context.base_step_duration,
                *context.step_duration_increase,
                *context.starting_step_duration,
                *context.base_foot_lift,
                *context.starting_step_foot_lift,
                *context.max_forward_acceleration,
            );
        }
        if let WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping =
            self.walk_state
        {
            let planned_step = self.step_adjustment(
                *context.forward_foot_support_offset,
                *context.backward_foot_support_offset,
                *context.max_step_adjustment,
            );
            self.walk_cycle(
                planned_step,
                sensor_data.cycle_info.last_cycle_duration,
                support_foot.support_side,
                *context.minimal_step_duration,
            );
        }

        let (left_arm, right_arm) = self.calculate_arm_joints(*context.shoulder_pitch_factor);

        let (mut left_leg, mut right_leg) =
            self.calculate_leg_joints(*context.torso_offset, *context.walk_hip_height);

        if let WalkState::Walking(_) = self.walk_state {
            swing_foot_leveling(
                &mut left_leg,
                &mut right_leg,
                sensor_data.positions.left_leg,
                sensor_data.positions.right_leg,
                self.swing_side,
                *context.swing_foot_leveling_factor,
            );
        }

        balance_adjustment(
            &mut left_leg,
            &mut right_leg,
            ground_contact,
            self.filtered_gyro_y.state(),
            *context.gyro_balance_factor,
        );

        context.walking_engine.on_subscription(|| self.clone());

        context.body_motion_safe_exits[BodyMotionType::Walk] =
            matches!(self.walk_state, WalkState::Standing);

        Ok(MainOutputs {
            walk_positions: Some(WalkPositions {
                positions: BodyJoints {
                    left_arm,
                    right_arm,
                    left_leg,
                    right_leg,
                },
            }),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn initialize_step_states_from_request(
        &mut self,
        walk_command: WalkCommand,
        support_side: Option<Side>,
        base_step_duration: Duration,
        step_duration_increase: Step,
        starting_step_duration: Duration,
        base_foot_lift: f32,
        starting_step_foot_lift: f32,
        max_forward_acceleration: f32,
    ) {
        self.walk_state = self.walk_state.next_walk_state(walk_command);
        let last_step = self.requested_step;

        match (self.walk_state, support_side) {
            (WalkState::Standing, _) | (_, None) => {
                self.requested_step = Step::zero();
                self.step_duration = Duration::ZERO;
                self.swing_side = Side::Left;
                self.max_swing_foot_lift = 0.0;
            }
            (WalkState::Starting(requested_step), Some(_)) => {
                self.requested_step = Step::zero();
                self.step_duration = starting_step_duration;
                let request_walk_to_left = requested_step.left > 0.0;
                self.swing_side = if request_walk_to_left {
                    Side::Right
                } else {
                    Side::Left
                };
                self.max_swing_foot_lift = starting_step_foot_lift;
            }
            (WalkState::Walking(requested_step), Some(support_side)) => {
                self.requested_step = requested_step;
                let duration_increase = Duration::from_secs_f32(
                    requested_step.forward.abs() * step_duration_increase.forward
                        + requested_step.left.abs() * step_duration_increase.left
                        + requested_step.turn.abs() * step_duration_increase.turn,
                );
                self.step_duration = base_step_duration + duration_increase;
                self.swing_side = support_side.opposite();
                self.max_swing_foot_lift = base_foot_lift;
            }
            (WalkState::Stopping, Some(support_side)) => {
                self.requested_step = Step::zero();
                self.step_duration = base_step_duration;
                self.swing_side = support_side.opposite();
                self.max_swing_foot_lift = base_foot_lift;
            }
        }

        let forward_accleration = self.requested_step.forward - last_step.forward;
        self.requested_step.forward =
            last_step.forward + forward_accleration.min(max_forward_acceleration);
    }

    fn next_foot_offsets(&mut self, planned_step: Step) {
        match self.swing_side {
            Side::Left => {
                let (support_foot, swing_foot, turn, support_foot_lift, swing_foot_lift) = self
                    .calculate_foot_offsets(planned_step, self.right_foot_t0, self.left_foot_t0);
                self.left_foot = swing_foot;
                self.right_foot = support_foot;
                self.turn = turn;
                self.left_foot_lift = swing_foot_lift;
                self.right_foot_lift = support_foot_lift;
            }
            Side::Right => {
                let (support_foot, swing_foot, turn, support_foot_lift, swing_foot_lift) = self
                    .calculate_foot_offsets(planned_step, self.left_foot_t0, self.right_foot_t0);
                self.left_foot = support_foot;
                self.right_foot = swing_foot;
                self.turn = turn;
                self.left_foot_lift = support_foot_lift;
                self.right_foot_lift = swing_foot_lift;
            }
        };
    }

    fn calculate_foot_offsets(
        &self,
        planned_step: Step,
        support_foot_t0: FootOffsets,
        swing_foot_t0: FootOffsets,
    ) -> (FootOffsets, FootOffsets, f32, f32, f32) {
        let linear_time = (self.t.as_secs_f32() / self.step_duration.as_secs_f32()).clamp(0.0, 1.0);
        let parabolic_time = parabolic_step(linear_time);

        let support_foot = FootOffsets {
            forward: support_foot_t0.forward
                + (-planned_step.forward / 2.0 - support_foot_t0.forward) * linear_time,
            left: support_foot_t0.left
                + (-planned_step.left / 2.0 - support_foot_t0.left) * linear_time,
        };

        let swing_foot = FootOffsets {
            forward: swing_foot_t0.forward
                + (planned_step.forward / 2.0 - swing_foot_t0.forward) * parabolic_time,
            left: swing_foot_t0.left
                + (planned_step.left / 2.0 - swing_foot_t0.left) * parabolic_time,
        };

        let turn_left_right = if self.swing_side == Side::Left {
            planned_step.turn
        } else {
            -1.0 * planned_step.turn
        };
        let turn = self.turn_t0 + (turn_left_right / 2.0 - self.turn_t0) * linear_time;

        let support_foot_lift = self.max_foot_lift_last_step
            * parabolic_return(
                (self.t_on_last_phase_end.as_secs_f32() / self.step_duration.as_secs_f32()
                    + linear_time)
                    .clamp(0.0, 1.0),
            );
        let swing_foot_lift = self.max_swing_foot_lift * parabolic_return(linear_time);

        (
            support_foot,
            swing_foot,
            turn,
            support_foot_lift,
            swing_foot_lift,
        )
    }

    fn end_step_phase(&mut self) {
        self.t_on_last_phase_end = self.t;
        self.max_foot_lift_last_step = self.max_swing_foot_lift;
        self.left_foot_t0 = self.left_foot;
        self.right_foot_t0 = self.right_foot;
        self.turn_t0 = self.turn;
    }

    fn step_adjustment(
        &self,
        forward_foot_support: f32,
        backward_foot_support: f32,
        max_adjustment: f32,
    ) -> Step {
        let filtered_torso_offset = self.filtered_torso_offset.state();

        let adjustment = if filtered_torso_offset > forward_foot_support {
            filtered_torso_offset - forward_foot_support
        } else if filtered_torso_offset < backward_foot_support {
            filtered_torso_offset - backward_foot_support
        } else {
            0.0
        };
        Step {
            forward: self.requested_step.forward
                + adjustment.clamp(-max_adjustment, max_adjustment),
            ..self.requested_step
        }
    }

    fn walk_cycle(
        &mut self,
        planned_step: Step,
        cycle_duration: Duration,
        support_side: Option<Side>,
        minimal_step_duration: Duration,
    ) {
        self.t += cycle_duration;
        self.next_foot_offsets(planned_step);
        let support_changed = match support_side {
            Some(support_side) => self.swing_side.opposite() != support_side,
            None => true,
        };
        if support_changed && self.t > minimal_step_duration {
            self.end_step_phase();
            self.t = Duration::ZERO;
        }
    }

    fn calculate_leg_joints(
        &self,
        torso_offset: f32,
        walk_hip_height: f32,
    ) -> (LegJoints, LegJoints) {
        let left_foot_to_robot = calculate_foot_to_robot(
            Side::Left,
            self.left_foot,
            self.turn,
            self.left_foot_lift,
            torso_offset,
            walk_hip_height,
        );
        let right_foot_to_robot = calculate_foot_to_robot(
            Side::Right,
            self.right_foot,
            self.turn,
            self.right_foot_lift,
            torso_offset,
            walk_hip_height,
        );
        let (is_reachable, left_leg, right_leg) =
            kinematics::leg_angles(left_foot_to_robot, right_foot_to_robot);
        if !is_reachable {
            warn!("Not reachable!");
        }
        (left_leg, right_leg)
    }

    fn calculate_arm_joints(&self, shoulder_pitch_factor: f32) -> (ArmJoints, ArmJoints) {
        let left_arm = ArmJoints {
            shoulder_pitch: PI / 2.0 + self.left_foot.forward * shoulder_pitch_factor,
            shoulder_roll: 0.3,
            elbow_yaw: 0.0,
            elbow_roll: 0.0,
            wrist_yaw: 0.0,
            hand: 0.0,
        };
        let right_arm = ArmJoints {
            shoulder_pitch: PI / 2.0 + self.right_foot.forward * shoulder_pitch_factor,
            shoulder_roll: -0.3,
            elbow_yaw: 0.0,
            elbow_roll: 0.0,
            wrist_yaw: 0.0,
            hand: 0.0,
        };
        (left_arm, right_arm)
    }
}

fn calculate_foot_to_robot(
    side: Side,
    foot: FootOffsets,
    turn_left_right: f32,
    foot_lift: f32,
    torso_offset: f32,
    walk_hip_height: f32,
) -> Isometry3<f32> {
    let hip_to_robot = match side {
        Side::Left => Isometry3::from(RobotDimensions::ROBOT_TO_LEFT_PELVIS),
        Side::Right => Isometry3::from(RobotDimensions::ROBOT_TO_RIGHT_PELVIS),
    };
    let foot_rotation = match side {
        Side::Left => turn_left_right,
        Side::Right => -turn_left_right,
    };
    hip_to_robot
        * Isometry3::translation(
            foot.forward - torso_offset,
            foot.left,
            -walk_hip_height + foot_lift,
        )
        * Isometry3::rotation(Vector3::z() * foot_rotation)
}

fn parabolic_return(x: f32) -> f32 {
    if x < 0.25 {
        return 8.0 * x * x;
    }
    if x < 0.75 {
        let x = x - 0.5;
        return 1.0 - 8.0 * x * x;
    }
    let x = 1.0 - x;
    8.0 * x * x
}

fn parabolic_step(x: f32) -> f32 {
    if x < 0.5 {
        2.0 * x * x
    } else {
        4.0 * x - 2.0 * x * x - 1.0
    }
}

fn balance_adjustment(
    left_leg: &mut LegJoints,
    right_leg: &mut LegJoints,
    ground_contact: &GroundContact,
    gyro_y: f32,
    gyro_balance_factor: f32,
) {
    // gyro balancing to compensate the wobble
    let gyro_adjustment = gyro_balance_factor * gyro_y;

    if ground_contact.left_foot {
        left_leg.ankle_pitch += gyro_adjustment
    }
    if ground_contact.right_foot {
        right_leg.ankle_pitch += gyro_adjustment
    }
}

fn swing_foot_leveling(
    left_leg: &mut LegJoints,
    right_leg: &mut LegJoints,
    measured_left_leg: LegJoints,
    measured_right_leg: LegJoints,
    swing_side: Side,
    swing_foot_leveling_factor: f32,
) {
    let (swing_leg, support_leg) = match swing_side {
        Side::Left => (left_leg, right_leg),
        Side::Right => (right_leg, left_leg),
    };
    let (_measured_swing_leg, measured_support_leg) = match swing_side {
        Side::Left => (measured_left_leg, measured_right_leg),
        Side::Right => (measured_right_leg, measured_left_leg),
    };

    let support_foot_pitch_error = measured_support_leg.ankle_pitch - support_leg.ankle_pitch;
    swing_leg.ankle_pitch += swing_foot_leveling_factor * support_foot_pitch_error;
}
