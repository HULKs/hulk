use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use log::warn;
use nalgebra::{Isometry3, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use types::{
    parameters::{KickSteps, WalkingEngine as WalkingEngineParameters},
    ArmJoints, BodyJoints, BodyJointsCommand, CycleTime, InertialMeasurementUnitData, Joints,
    KickVariant, LegJoints, MotionCommand, MotionSafeExits, MotionType, RobotKinematics,
    SensorData, Side, Step, StepAdjustment, WalkCommand,
};

use self::{
    arms::SwingingArm,
    balancing::{step_adjustment, support_leg_gyro_balancing, swing_leg_foot_leveling},
    engine::{calculate_foot_to_robot, parabolic_return, parabolic_step},
    foot_offsets::FootOffsets,
    kicking::apply_joint_overrides,
    walk_state::WalkState,
};

mod arms;
mod balancing;
mod engine;
mod foot_offsets;
mod kicking;
mod walk_state;

/// # WalkingEngine
/// This node generates foot positions and thus leg angles for the robot to execute a walk.
/// The algorithm to compute the feet trajectories is loosely based on the work of Bernhard Hengst
/// at the team rUNSWift. An explanation of this algorithm can be found in the team's research
/// report from 2014 (<http://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>).
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WalkingEngine {
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
    planned_step_duration: Duration,
    /// Fix the side of the swing foot for an entire walk phase
    swing_side: Side,
    /// Low pass filter the gyro for balance adjustment
    filtered_gyro: LowPassFilter<Vector2<f32>>,
    /// Low pass filter the imu pitch for balance adjustment
    filtered_imu_pitch: LowPassFilter<f32>,
    /// Low pass filter the robot tilt for step adjustments
    filtered_robot_tilt_shift: LowPassFilter<f32>,
    /// Foot offsets for the left foot the walking engine interpolation generated for the last cycle
    last_left_walk_request: FootOffsets,
    /// Foot offsets for the right foot the walking engine interpolation generated for the last cycle
    last_right_walk_request: FootOffsets,
    /// leg balancing adjustment for the left leg of the last cycle
    last_left_leg_adjustment: LegJoints<f32>,
    /// leg balancing adjustment for the right leg of the last cycle
    last_right_leg_adjustment: LegJoints<f32>,
    /// motion of the left arm currently executed
    left_arm: SwingingArm,
    /// motion of the right arm currently executed
    right_arm: SwingingArm,
    /// counting steps that exceeded a timeout
    number_of_timeouted_steps: usize,
    /// counting steps that had support changes that were not inside an accepted range
    number_of_unstable_steps: usize,
    /// number of steps walking has to make zero steps to stabilize before starting to walk again
    remaining_stabilizing_steps: usize,
}

#[context]
pub struct CreationContext {
    pub config: Parameter<WalkingEngineParameters, "walking_engine">,
    pub kick_steps: Parameter<KickSteps, "kick_steps">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,
}

#[context]
#[derive(Debug)]
pub struct CycleContext {
    pub step_adjustment: AdditionalOutput<StepAdjustment, "step_adjustment">,
    pub planned_step_duration: AdditionalOutput<Duration, "walking_engine.planned_step_duration">,
    pub t: AdditionalOutput<Duration, "walking_engine.t">,
    pub t_on_last_phase_end: AdditionalOutput<Duration, "walking_engine.t_on_last_phase_end">,
    // TODO: ask hendrik how to do that
    // pub walking_engine: AdditionalOutput<WalkingEngine, "walking_engine">,
    pub config: Parameter<WalkingEngineParameters, "walking_engine">,
    pub kick_steps: Parameter<KickSteps, "kick_steps">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,

    pub motion_command: Input<MotionCommand, "motion_command">,
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub walk_command: Input<WalkCommand, "walk_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_joints_command: MainOutput<BodyJointsCommand<f32>>,
}

impl WalkingEngine {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                Vector2::default(),
                context.config.gyro_low_pass_factor,
            ),
            filtered_imu_pitch: LowPassFilter::with_smoothing_factor(
                0.0,
                context.config.imu_pitch_low_pass_factor,
            ),
            filtered_robot_tilt_shift: LowPassFilter::with_smoothing_factor(
                0.0,
                context.config.tilt_shift_low_pass_factor,
            ),
            left_arm: SwingingArm::new(Side::Left),
            right_arm: SwingingArm::new(Side::Right),
            ..Default::default()
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        self.filtered_gyro.update(
            context
                .sensor_data
                .inertial_measurement_unit
                .angular_velocity
                .xy(),
        );
        self.filtered_imu_pitch
            .update(context.sensor_data.inertial_measurement_unit.roll_pitch.y);
        self.filter_robot_tilt_shift(
            context.robot_kinematics,
            &context.sensor_data.inertial_measurement_unit,
        );

        let is_step_started_this_cycle = self.t.is_zero();
        if *context.has_ground_contact {
            if is_step_started_this_cycle {
                self.initialize_step_states_from_request(
                    *context.walk_command,
                    self.swing_side,
                    context.config,
                    context.kick_steps,
                );
            }
        } else {
            self.walk_state = WalkState::Standing;
        }

        match &self.walk_state {
            WalkState::Standing => self.reset(),
            WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping => {
                self.walk_cycle(
                    context.cycle_time.last_cycle_duration,
                    context.config,
                    &mut context.step_adjustment,
                );
            }
            WalkState::Kicking(..) => self.kick_cycle(last_cycle_duration),
        }

        let left_foot_pressure = context.sensor_data.force_sensitive_resistors.left.sum();
        let right_foot_pressure = context.sensor_data.force_sensitive_resistors.right.sum();
        let has_support_changed = match self.swing_side {
            Side::Left => left_foot_pressure > context.config.foot_pressure_threshold,
            Side::Right => right_foot_pressure > context.config.foot_pressure_threshold,
        };
        if has_support_changed && self.t > context.config.minimal_step_duration {
            let deviation_from_plan = self
                .t
                .checked_sub(self.planned_step_duration)
                .unwrap_or_else(|| self.planned_step_duration.checked_sub(self.t).unwrap());
            if deviation_from_plan > context.config.stable_step_deviation {
                self.number_of_unstable_steps += 1;
            } else {
                self.number_of_unstable_steps = 0;
            }
            self.number_of_timeouted_steps = 0;
            self.end_step_phase();
        } else if self.t > context.config.maximal_step_duration {
            self.number_of_timeouted_steps += 1;
            self.end_step_phase();
        }

        let left_arm = self.left_arm.next(
            self.left_foot,
            context.motion_command,
            last_cycle_duration,
            &context.config.swinging_arms,
        )?;
        let right_arm = self.right_arm.next(
            self.right_foot,
            context.motion_command,
            last_cycle_duration,
            &context.config.swinging_arms,
        )?;

        let arm_compensation = self
            .left_arm
            .torso_tilt_compensation(&context.config.swinging_arms)?
            + self
                .right_arm
                .torso_tilt_compensation(&context.config.swinging_arms)?;

        let (mut left_leg, mut right_leg) = self.calculate_leg_joints(
            context.config.torso_shift_offset,
            context.config.walk_hip_height,
        );
        left_leg.hip_pitch += arm_compensation - context.config.torso_tilt_offset;
        right_leg.hip_pitch += arm_compensation - context.config.torso_tilt_offset;

        if let WalkState::Kicking(kick_variant, _, kick_step_i, strength) = self.walk_state {
            let swing_leg = match self.swing_side {
                Side::Left => &mut left_leg,
                Side::Right => &mut right_leg,
            };
            let kick_steps = match kick_variant {
                KickVariant::Forward => &context.kick_steps.forward,
                KickVariant::Turn => &context.kick_steps.turn,
                KickVariant::Side => &context.kick_steps.side,
            };
            let kick_step = &kick_steps[kick_step_i];
            apply_joint_overrides(kick_step, swing_leg, self.t, strength);
        }

        let mut support_leg_adjustment = LegJoints::default();
        let mut swing_leg_adjustment = LegJoints::default();

        if let WalkState::Walking(_) = self.walk_state {
            let swing_leg_foot_leveling = swing_leg_foot_leveling(
                &left_leg,
                &right_leg,
                context.sensor_data.positions.left_leg,
                context.sensor_data.positions.right_leg,
                self.filtered_imu_pitch.state(),
                self.swing_side,
                context.config,
                self.t,
                self.planned_step_duration,
            );
            swing_leg_adjustment = swing_leg_adjustment + swing_leg_foot_leveling;
        }
        if let WalkState::Walking(_) | WalkState::Kicking(..) = self.walk_state {
            let support_leg_gyro_balancing = support_leg_gyro_balancing(
                self.filtered_gyro.state(),
                context.config.gyro_balance_factors,
            );
            support_leg_adjustment = support_leg_adjustment + support_leg_gyro_balancing;
        }

        adjust_legs(
            &mut left_leg,
            &mut right_leg,
            support_leg_adjustment,
            swing_leg_adjustment,
            self.swing_side,
            &mut self.last_left_leg_adjustment,
            &mut self.last_right_leg_adjustment,
            context.config.max_leg_adjustment_velocity,
        );

        context
            .planned_step_duration
            .fill_if_subscribed(|| self.planned_step_duration);
        context.t.fill_if_subscribed(|| self.t);
        context
            .t_on_last_phase_end
            .fill_if_subscribed(|| self.t_on_last_phase_end);
        // TODO: refill
        // context.walking_engine.fill_on_subscription(|| self.clone());

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
            walk_joints_command: BodyJointsCommand {
                positions: BodyJoints {
                    left_arm,
                    right_arm,
                    left_leg,
                    right_leg,
                },
                stiffnesses,
            }
            .into(),
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
        swing_side: Side,
        config: &WalkingEngineParameters,
        kick_steps: &KickSteps,
    ) {
        self.left_foot_t0 = self.left_foot;
        self.right_foot_t0 = self.right_foot;
        self.turn_t0 = self.turn;
        self.walk_state =
            self.walk_state
                .next_walk_state(walk_command, self.swing_side, kick_steps);

        if self.number_of_timeouted_steps >= config.max_number_of_timeouted_steps {
            self.current_step = config.emergency_step;
            self.planned_step_duration = config.emergency_step_duration;
            self.swing_side = swing_side;
            self.max_swing_foot_lift = config.emergency_foot_lift;
            self.number_of_timeouted_steps = 0;
            return;
        }

        if self.number_of_unstable_steps >= config.max_number_of_unstable_steps {
            self.number_of_unstable_steps = 0;
            self.remaining_stabilizing_steps = config.number_of_stabilizing_steps;
        }
        if self.remaining_stabilizing_steps > 0 {
            self.remaining_stabilizing_steps -= 1;
            self.current_step = Step::zero();
            self.planned_step_duration = config.base_step_duration;
            self.swing_side = swing_side.opposite();
            self.max_swing_foot_lift = config.base_foot_lift;
            return;
        }

        let last_step = self.current_step;
        match self.walk_state {
            WalkState::Standing => {
                self.current_step = Step::zero();
                self.planned_step_duration = Duration::ZERO;
                self.swing_side = Side::Left;
                self.max_swing_foot_lift = 0.0;
            }
            WalkState::Starting(_) => {
                self.current_step = Step::zero();
                self.planned_step_duration = config.starting_step_duration;
                self.swing_side = swing_side.opposite();
                self.max_swing_foot_lift = config.starting_step_foot_lift;
            }
            WalkState::Walking(requested_step) => {
                let next_support_side = swing_side;
                let next_swing_side = swing_side.opposite();
                let requested_step = clamp_to_anatomic_constraints(
                    requested_step,
                    next_support_side,
                    config.inside_turn_ratio,
                );
                let forward_acceleration = requested_step.forward - last_step.forward;
                self.current_step = Step {
                    forward: last_step.forward
                        + forward_acceleration.min(config.max_forward_acceleration),
                    ..requested_step
                };
                let absolute_next_step = Step {
                    forward: requested_step.forward.abs(),
                    left: requested_step.left.abs(),
                    turn: requested_step.turn.abs(),
                };

                let step_duration_increase = absolute_next_step * config.step_duration_increase;
                let duration_increase = Duration::from_secs_f32(step_duration_increase.sum());
                self.planned_step_duration = config.base_step_duration + duration_increase;

                self.swing_side = next_swing_side;

                let step_foot_lift_increase = absolute_next_step * config.step_foot_lift_increase;
                self.max_swing_foot_lift = config.base_foot_lift + step_foot_lift_increase.sum();
            }
            WalkState::Stopping => {
                self.current_step = Step::zero();
                self.planned_step_duration = config.base_step_duration;
                self.swing_side = swing_side.opposite();
                self.max_swing_foot_lift = config.base_foot_lift;
            }
            WalkState::Kicking(kick_variant, kick_side, kick_step_i, _) => {
                let kick_steps = match kick_variant {
                    KickVariant::Forward => &kick_steps.forward,
                    KickVariant::Turn => &kick_steps.turn,
                    KickVariant::Side => &kick_steps.side,
                };
                let base_step = kick_steps[kick_step_i].base_step;
                self.current_step = match kick_side {
                    Side::Left => base_step,
                    Side::Right => base_step.mirrored(),
                };
                self.planned_step_duration = config.base_step_duration;
                self.swing_side = swing_side.opposite();
                self.max_swing_foot_lift = config.base_foot_lift + config.additional_kick_foot_lift;
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
        self.planned_step_duration = Duration::ZERO;
        self.swing_side = Side::Left;
        self.filtered_gyro.reset(Vector2::default());
        self.filtered_imu_pitch.reset(0.0);
        self.filtered_robot_tilt_shift.reset(0.0);
        self.last_left_walk_request = FootOffsets::zero();
        self.last_right_walk_request = FootOffsets::zero();
        self.last_left_leg_adjustment = LegJoints::default();
        self.last_right_leg_adjustment = LegJoints::default();
        self.number_of_timeouted_steps = 0;
        self.number_of_unstable_steps = 0;
        self.remaining_stabilizing_steps = 0;
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
        let linear_time =
            (self.t.as_secs_f32() / self.planned_step_duration.as_secs_f32()).clamp(0.0, 1.0);
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
                (self.t_on_last_phase_end.as_secs_f32() / self.planned_step_duration.as_secs_f32()
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

    fn walk_cycle(
        &mut self,
        cycle_duration: Duration,
        config: &WalkingEngineParameters,
        step_adjustment_output: &mut AdditionalOutput<StepAdjustment>,
    ) {
        self.t += cycle_duration;
        let (
            next_left_walk_request,
            next_right_walk_request,
            next_turn,
            next_left_foot_lift,
            next_right_foot_lift,
        ) = self.next_foot_offsets(self.current_step);
        let (
            adjusted_left_foot,
            adjusted_right_foot,
            adjusted_left_foot_lift,
            adjusted_right_foot_lift,
            adjusted_remaining_steps,
        ) = step_adjustment(
            self.t,
            self.planned_step_duration,
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
            step_adjustment_output,
            next_left_foot_lift,
            next_right_foot_lift,
            config.stabilization_foot_lift_multiplier,
            config.stabilization_foot_lift_offset,
            self.remaining_stabilizing_steps,
        );
        self.last_left_walk_request = next_left_walk_request;
        self.last_right_walk_request = next_right_walk_request;
        self.left_foot = adjusted_left_foot;
        self.right_foot = adjusted_right_foot;
        self.turn = next_turn;
        self.left_foot_lift = adjusted_left_foot_lift;
        self.right_foot_lift = adjusted_right_foot_lift;
        self.remaining_stabilizing_steps = adjusted_remaining_steps
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
        torso_shift_offset: f32,
        walk_hip_height: f32,
    ) -> (LegJoints<f32>, LegJoints<f32>) {
        let left_foot_to_robot = calculate_foot_to_robot(
            Side::Left,
            self.left_foot,
            self.turn,
            self.left_foot_lift,
            torso_shift_offset,
            walk_hip_height,
        );
        let right_foot_to_robot = calculate_foot_to_robot(
            Side::Right,
            self.right_foot,
            self.turn,
            self.right_foot_lift,
            torso_shift_offset,
            walk_hip_height,
        );
        let (is_reachable, left_leg, right_leg) =
            kinematics::leg_angles(left_foot_to_robot, right_foot_to_robot);
        if !is_reachable {
            warn!("Not reachable!");
        }
        (left_leg, right_leg)
    }
}

#[allow(clippy::too_many_arguments)]
fn adjust_legs(
    left_leg: &mut LegJoints<f32>,
    right_leg: &mut LegJoints<f32>,
    support_leg_adjustment: LegJoints<f32>,
    swing_leg_adjustment: LegJoints<f32>,
    swing_side: Side,
    last_left_leg_adjustment: &mut LegJoints<f32>,
    last_right_leg_adjustment: &mut LegJoints<f32>,
    max_leg_adjustment_velocity: LegJoints<f32>,
) {
    let (left_leg_adjustment, right_leg_adjustment) = match swing_side {
        Side::Left => (swing_leg_adjustment, support_leg_adjustment),
        Side::Right => (support_leg_adjustment, swing_leg_adjustment),
    };
    let limited_left_leg_adjustment = *last_left_leg_adjustment
        + (left_leg_adjustment - *last_left_leg_adjustment).clamp(
            max_leg_adjustment_velocity * -1.0,
            max_leg_adjustment_velocity,
        );
    let limited_right_leg_adjustment = *last_right_leg_adjustment
        + (right_leg_adjustment - *last_right_leg_adjustment).clamp(
            max_leg_adjustment_velocity * -1.0,
            max_leg_adjustment_velocity,
        );

    *left_leg = *left_leg + limited_left_leg_adjustment;
    *right_leg = *right_leg + limited_right_leg_adjustment;

    *last_left_leg_adjustment = limited_left_leg_adjustment;
    *last_right_leg_adjustment = limited_right_leg_adjustment;
}

fn clamp_to_anatomic_constraints(
    request: Step,
    support_side: Side,
    inside_turn_ratio: f32,
) -> Step {
    let sideways_direction = if request.left.is_sign_positive() {
        Side::Left
    } else {
        Side::Right
    };
    let clamped_left = if sideways_direction == support_side {
        0.0
    } else {
        request.left
    };
    let turn_direction = if request.turn.is_sign_positive() {
        Side::Left
    } else {
        Side::Right
    };
    let turn_ratio = if turn_direction == support_side {
        inside_turn_ratio
    } else {
        1.0 - inside_turn_ratio
    };
    let clamped_turn = turn_ratio * request.turn;
    Step {
        forward: request.forward,
        left: clamped_left,
        turn: clamped_turn,
    }
}
