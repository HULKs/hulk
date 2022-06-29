use std::{f32::consts::PI, time::Duration};

use anyhow::Result;
use log::warn;
use macros::{module, require_some, SerializeHierarchy};
use nalgebra::{geometry::Isometry3, Point3, Vector3};
use serde::{Deserialize, Serialize};

use crate::{
    control::filtering::LowPassFilter,
    framework::configuration,
    kinematics,
    types::{
        ArmJoints, BodyJoints, BodyJointsCommand, InertialMeasurementUnitData, Joints, KickVariant,
        LegJoints, MotionSafeExits, MotionType, RobotKinematics, SensorData, Side, Step,
        SupportFoot, WalkCommand,
    },
};

use self::{
    balancing::{foot_leveling, gyro_balancing, step_adjustment},
    engine::{calculate_foot_to_robot, parabolic_return, parabolic_step},
    foot_offsets::FootOffsets,
    kicking::apply_joint_overrides,
    walk_state::WalkState,
};

mod balancing;
mod engine;
mod foot_offsets;
mod kicking;
mod walk_state;

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
    current_step: Step,
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
    /// Low pass filter the robot tilt for step adjustments
    filtered_robot_tilt_shift: LowPassFilter<f32>,
    /// Foot offsets for the left foot the walking engine interpolation generated for the last cycle
    last_left_walk_request: FootOffsets,
    /// Foot offsets for the right foot the walking engine interpolation generated for the last cycle
    last_right_walk_request: FootOffsets,
    /// step adjustment for the left foot of the last cycle
    last_left_level_adjustment: f32,
    /// step adjustment for the right foot of the last cycle
    last_right_level_adjustment: f32,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = walk_command, data_type = WalkCommand)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[persistent_state(path = walk_return_offset, data_type = Step)]
#[parameter(path = control.walking_engine, data_type = configuration::WalkingEngine, name = config)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[additional_output(path = walking_engine, data_type = WalkingEngine)]
#[main_output(name = walk_joints_command, data_type = BodyJointsCommand)]
impl WalkingEngine {}

impl WalkingEngine {
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            filtered_gyro_y: LowPassFilter::with_alpha(0.0, context.config.gyro_low_pass_factor),
            filtered_robot_tilt_shift: LowPassFilter::with_alpha(0.0, 0.7),
            ..Default::default()
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let support_foot = require_some!(context.support_foot);
        let walk_command = require_some!(context.walk_command);
        let robot_kinematics = require_some!(context.robot_kinematics);

        self.filtered_gyro_y
            .update(sensor_data.inertial_measurement_unit.angular_velocity.y);
        self.filter_robot_tilt_shift(robot_kinematics, &sensor_data.inertial_measurement_unit);

        if self.t.is_zero() {
            self.initialize_step_states_from_request(
                *walk_command,
                support_foot.support_side,
                context.config,
            );
        }
        let has_support_changed = match support_foot.support_side {
            Some(support_side) => self.swing_side.opposite() != support_side,
            None => true,
        };
        match &self.walk_state {
            WalkState::Standing => self.reset(),
            WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping => {
                self.walk_cycle(sensor_data.cycle_info.last_cycle_duration, context.config);
            }
            WalkState::Kicking(..) => self.kick_cycle(sensor_data.cycle_info.last_cycle_duration),
        }
        if has_support_changed && self.t > context.config.minimal_step_duration {
            self.end_step_phase();
        }

        let (left_arm, right_arm) = self.calculate_arm_joints(context.config.shoulder_pitch_factor);

        let (mut left_leg, mut right_leg) =
            self.calculate_leg_joints(context.config.torso_offset, context.config.walk_hip_height);

        if let WalkState::Walking(_) = self.walk_state {
            foot_leveling(
                &mut left_leg,
                &mut right_leg,
                sensor_data.positions.left_leg,
                sensor_data.positions.right_leg,
                sensor_data.inertial_measurement_unit.roll_pitch.y,
                self.swing_side,
                &mut self.last_left_level_adjustment,
                &mut self.last_right_level_adjustment,
                context.config,
            );
        } else if let WalkState::Kicking(kick_variant, _, kick_step_i) = self.walk_state {
            let swing_leg = match self.swing_side {
                Side::Left => &mut left_leg,
                Side::Right => &mut right_leg,
            };
            let kick_steps = match kick_variant {
                KickVariant::Forward => &context.config.forward_kick_steps,
                KickVariant::Turn => &context.config.turn_kick_steps,
            };
            let kick_step = &kick_steps[kick_step_i];
            apply_joint_overrides(kick_step, swing_leg, self.t);
        }

        if let WalkState::Walking(_) | WalkState::Kicking(..) = self.walk_state {
            let support_leg = match self.swing_side {
                Side::Left => &mut right_leg,
                Side::Right => &mut left_leg,
            };
            gyro_balancing(
                support_leg,
                self.filtered_gyro_y.state(),
                context.config.gyro_balance_factor,
            );
        }

        context.walking_engine.fill_on_subscription(|| self.clone());

        *context.walk_return_offset = match self.swing_side {
            Side::Left => Step {
                forward: self.left_foot.forward,
                left: self.left_foot.left,
                turn: self.turn,
            },
            Side::Right => Step {
                forward: self.right_foot.forward,
                left: self.right_foot.left,
                turn: self.turn,
            },
        };
        context.motion_safe_exits[MotionType::Walk] =
            matches!(self.walk_state, WalkState::Standing);

        let leg_stiffness = match self.walk_state {
            WalkState::Standing => context.config.leg_stiffness_stand,
            WalkState::Starting(_)
            | WalkState::Walking(_)
            | WalkState::Kicking(..)
            | WalkState::Stopping => context.config.leg_stiffness_walk,
        };
        let stiffnesses = BodyJoints {
            left_arm: ArmJoints::fill(context.config.arm_stiffness),
            right_arm: ArmJoints::fill(context.config.arm_stiffness),
            left_leg: LegJoints::fill(leg_stiffness),
            right_leg: LegJoints::fill(leg_stiffness),
        };

        Ok(MainOutputs {
            walk_joints_command: Some(BodyJointsCommand {
                positions: BodyJoints {
                    left_arm,
                    right_arm,
                    left_leg,
                    right_leg,
                },
                stiffnesses,
            }),
        })
    }

    fn filter_robot_tilt_shift(
        &mut self,
        robot_kinematics: &RobotKinematics,
        imu: &InertialMeasurementUnitData,
    ) {
        let robot_height = match self.swing_side.opposite() {
            Side::Left => robot_kinematics.left_sole_to_robot.translation.z,
            Side::Right => robot_kinematics.right_sole_to_robot.translation.z,
        };
        let robot_rotation = Isometry3::rotation(Vector3::y() * imu.roll_pitch.y)
            * Isometry3::rotation(Vector3::x() * imu.roll_pitch.x);
        let robot_projected_to_ground =
            robot_rotation.inverse() * Isometry3::translation(0.0, 0.0, robot_height);
        let measured_robot_tilt_shift = (robot_projected_to_ground * Point3::origin()).x;
        self.filtered_robot_tilt_shift
            .update(measured_robot_tilt_shift);
    }

    fn initialize_step_states_from_request(
        &mut self,
        walk_command: WalkCommand,
        measured_support_side: Option<Side>,
        config: &configuration::WalkingEngine,
    ) {
        self.left_foot_t0 = self.left_foot;
        self.right_foot_t0 = self.right_foot;
        self.turn_t0 = self.turn;
        let last_step = self.current_step;
        self.walk_state = self
            .walk_state
            .next_walk_state(walk_command, self.swing_side, config);

        match (self.walk_state, measured_support_side) {
            (WalkState::Standing, _) | (_, None) => {
                self.current_step = Step::zero();
                self.step_duration = Duration::ZERO;
                self.swing_side = Side::Left;
                self.max_swing_foot_lift = 0.0;
            }
            (WalkState::Starting(requested_step), Some(_)) => {
                self.current_step = Step::zero();
                self.step_duration = config.starting_step_duration;
                let request_walk_to_left = requested_step.left > 0.0;
                self.swing_side = if request_walk_to_left {
                    Side::Right
                } else {
                    Side::Left
                };
                self.max_swing_foot_lift = config.starting_step_foot_lift;
            }
            (WalkState::Walking(requested_step), Some(support_side)) => {
                let forward_acceleration = requested_step.forward - last_step.forward;
                self.current_step = Step {
                    forward: last_step.forward
                        + forward_acceleration.min(config.max_forward_acceleration),
                    ..requested_step
                };
                let duration_increase = Duration::from_secs_f32(
                    requested_step.forward.abs() * config.step_duration_increase.forward
                        + requested_step.left.abs() * config.step_duration_increase.left
                        + requested_step.turn.abs() * config.step_duration_increase.turn,
                );
                self.step_duration = config.base_step_duration + duration_increase;
                self.swing_side = support_side.opposite();
                self.max_swing_foot_lift = config.base_foot_lift;
            }
            (WalkState::Stopping, Some(support_side)) => {
                self.current_step = Step::zero();
                self.step_duration = config.base_step_duration;
                self.swing_side = support_side.opposite();
                self.max_swing_foot_lift = config.base_foot_lift;
            }
            (WalkState::Kicking(kick_variant, kick_side, kick_step_i), Some(support_side)) => {
                let kick_steps = match kick_variant {
                    KickVariant::Forward => &config.forward_kick_steps,
                    KickVariant::Turn => &config.turn_kick_steps,
                };
                let base_step = kick_steps[kick_step_i].base_step;
                self.current_step = match kick_side {
                    Side::Left => base_step,
                    Side::Right => base_step.mirrored(),
                };
                self.step_duration = config.base_step_duration;
                self.swing_side = support_side.opposite();
                self.max_swing_foot_lift = config.base_foot_lift;
            }
        }
    }

    fn reset(&mut self) {
        self.current_step = Step::zero();
        self.max_swing_foot_lift = 0.0;
        self.left_foot = FootOffsets::zero();
        self.right_foot = FootOffsets::zero();
        self.turn = 0.0;
        self.left_foot_t0 = FootOffsets::zero();
        self.right_foot_t0 = FootOffsets::zero();
        self.turn_t0 = 0.0;
        self.left_foot_lift = 0.0;
        self.right_foot_lift = 0.0;
        self.max_foot_lift_last_step = 0.0;
        self.t = Duration::ZERO;
        self.t_on_last_phase_end = Duration::ZERO;
        self.step_duration = Duration::ZERO;
        self.swing_side = Side::Left;
        self.filtered_gyro_y.reset(0.0);
        self.filtered_robot_tilt_shift.reset(0.0);
        self.last_left_walk_request = FootOffsets::zero();
        self.last_right_walk_request = FootOffsets::zero();
        self.last_left_level_adjustment = 0.0;
        self.last_right_level_adjustment = 0.0;
    }

    fn next_foot_offsets(
        &mut self,
        planned_step: Step,
    ) -> (FootOffsets, FootOffsets, f32, f32, f32) {
        match self.swing_side {
            Side::Left => {
                let (support_foot, swing_foot, turn, support_foot_lift, swing_foot_lift) = self
                    .calculate_foot_offsets(planned_step, self.right_foot_t0, self.left_foot_t0);
                (
                    swing_foot,
                    support_foot,
                    turn,
                    swing_foot_lift,
                    support_foot_lift,
                )
            }
            Side::Right => {
                let (support_foot, swing_foot, turn, support_foot_lift, swing_foot_lift) = self
                    .calculate_foot_offsets(planned_step, self.left_foot_t0, self.right_foot_t0);
                (
                    support_foot,
                    swing_foot,
                    turn,
                    support_foot_lift,
                    swing_foot_lift,
                )
            }
        }
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
        self.t = Duration::ZERO;
        self.max_foot_lift_last_step = self.max_swing_foot_lift;
        self.last_left_walk_request = self.left_foot;
        self.last_right_walk_request = self.right_foot;
    }

    fn walk_cycle(&mut self, cycle_duration: Duration, config: &configuration::WalkingEngine) {
        self.t += cycle_duration;
        let (
            next_left_walk_request,
            next_right_walk_request,
            next_turn,
            next_left_foot_lift,
            next_right_foot_lift,
        ) = self.next_foot_offsets(self.current_step);
        let (adjusted_left_foot, adjusted_right_foot) = step_adjustment(
            self.swing_side,
            self.filtered_robot_tilt_shift.state(),
            self.left_foot,
            self.right_foot,
            next_left_walk_request,
            next_right_walk_request,
            self.last_left_walk_request,
            self.last_right_walk_request,
            config.forward_foot_support_offset,
            config.backward_foot_support_offset,
            config.max_step_adjustment,
        );
        self.last_left_walk_request = next_left_walk_request;
        self.last_right_walk_request = next_right_walk_request;
        self.left_foot = adjusted_left_foot;
        self.right_foot = adjusted_right_foot;
        self.turn = next_turn;
        self.left_foot_lift = next_left_foot_lift;
        self.right_foot_lift = next_right_foot_lift;
    }

    fn kick_cycle(&mut self, cycle_duration: Duration) {
        self.t += cycle_duration;
        let (
            next_left_walk_request,
            next_right_walk_request,
            next_turn,
            next_left_foot_lift,
            next_right_foot_lift,
        ) = self.next_foot_offsets(self.current_step);
        self.left_foot = next_left_walk_request;
        self.right_foot = next_right_walk_request;
        self.turn = next_turn;
        self.left_foot_lift = next_left_foot_lift;
        self.right_foot_lift = next_right_foot_lift;
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
