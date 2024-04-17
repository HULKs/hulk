use std::f32::consts::FRAC_PI_2;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot, Walk};
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use kinematics::forward;
use linear_algebra::{vector, Isometry3, Orientation3, Point3, Vector3};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::body::BodyJoints,
    motion_selection::{MotionSafeExits, MotionType},
    motor_commands::MotorCommands,
    obstacle_avoiding_arms::{ArmCommand, ArmCommands},
    sensor_data::SensorData,
    step_plan::Step,
    support_foot::Side,
    walk_command::WalkCommand,
};
use walking_engine::{kick_steps::KickSteps, parameters::Parameters, Context, Engine};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkingEngine {
    engine: Engine,
    last_actuated_joints: BodyJoints,
    filtered_gyro: LowPassFilter<nalgebra::Vector3<f32>>,
}

#[context]
pub struct CreationContext {
    parameters: Parameter<Parameters, "walking_engine">,
}

#[context]
#[derive(Debug)]
pub struct CycleContext {
    parameters: Parameter<Parameters, "walking_engine">,
    kick_steps: Parameter<KickSteps, "kick_steps">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    walk_return_offset: CyclerState<Step, "walk_return_offset">,

    cycle_time: Input<CycleTime, "cycle_time">,
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    sensor_data: Input<SensorData, "sensor_data">,
    walk_command: Input<WalkCommand, "walk_command">,
    robot_to_ground: Input<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
    obstacle_avoiding_arms: Input<ArmCommands, "obstacle_avoiding_arms">,

    debug_output: AdditionalOutput<Engine, "walking.engine">,
    last_actuated_joints: AdditionalOutput<BodyJoints, "walking.last_actuated_joints">,
    robot_to_walk: AdditionalOutput<Isometry3<Robot, Walk>, "walking.robot_to_walk">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_motor_commands: MainOutput<MotorCommands<BodyJoints<f32>>>,
}

impl WalkingEngine {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            engine: Engine::default(),
            last_actuated_joints: Default::default(),
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                nalgebra::Vector3::zeros(),
                context.parameters.gyro_balancing.low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, mut cycle_context: CycleContext) -> Result<MainOutputs> {
        self.filtered_gyro.update(
            cycle_context
                .sensor_data
                .inertial_measurement_unit
                .angular_velocity,
        );

        let torso_tilt_compensation_factor = cycle_context
            .parameters
            .swinging_arms
            .torso_tilt_compensation_factor;

        let arm_compensation = compensate_arm_motion_with_torso_tilt(
            &cycle_context.obstacle_avoiding_arms.left_arm,
            torso_tilt_compensation_factor,
        ) + compensate_arm_motion_with_torso_tilt(
            &cycle_context.obstacle_avoiding_arms.right_arm,
            torso_tilt_compensation_factor,
        );

        let robot_to_walk = Isometry3::from_parts(
            vector![
                cycle_context.parameters.base.torso_offset,
                0.0,
                cycle_context.parameters.base.walk_height,
            ],
            Orientation3::new(
                Vector3::y_axis() * (cycle_context.parameters.base.torso_tilt + arm_compensation),
            ),
        );

        let context = Context {
            parameters: cycle_context.parameters,
            kick_steps: cycle_context.kick_steps,
            cycle_time: cycle_context.cycle_time,
            center_of_mass: cycle_context.center_of_mass,
            sensor_data: cycle_context.sensor_data,
            robot_to_ground: cycle_context.robot_to_ground,
            gyro: self.filtered_gyro.state(),
            current_joints: self.last_actuated_joints,
            robot_to_walk,
            obstacle_avoiding_arms: cycle_context.obstacle_avoiding_arms,
        };

        match *cycle_context.walk_command {
            WalkCommand::Stand => self.engine.stand(&context),
            WalkCommand::Walk { step } => self.engine.walk(&context, step),
            WalkCommand::Kick {
                variant,
                side,
                strength,
            } => self.engine.kick(&context, variant, side, strength),
        };

        self.engine.tick(&context);

        let motor_commands = self.engine.compute_commands(&context);

        self.last_actuated_joints = motor_commands.positions;

        *cycle_context.walk_return_offset = self
            .calculate_return_offset(cycle_context.parameters, robot_to_walk)
            .unwrap_or_default();
        cycle_context.motion_safe_exits[MotionType::Walk] = self.engine.is_standing();

        cycle_context
            .debug_output
            .fill_if_subscribed(|| self.engine.clone());
        cycle_context
            .last_actuated_joints
            .fill_if_subscribed(|| self.last_actuated_joints);
        cycle_context
            .robot_to_walk
            .fill_if_subscribed(|| robot_to_walk);

        Ok(MainOutputs {
            walk_motor_commands: motor_commands.into(),
        })
    }

    fn calculate_return_offset(
        &self,
        parameters: &Parameters,
        robot_to_walk: Isometry3<Robot, Walk>,
    ) -> Option<Step> {
        let left_sole = robot_to_walk
            * forward::left_sole_to_robot(&self.last_actuated_joints.left_leg).as_pose();
        let right_sole = robot_to_walk
            * forward::right_sole_to_robot(&self.last_actuated_joints.right_leg).as_pose();
        let support_side = self.engine.support_side()?;
        let swing_sole = match support_side {
            Side::Left => right_sole,
            Side::Right => left_sole,
        };
        let swing_sole_base_offset = match support_side {
            Side::Left => parameters.base.foot_offset_right,
            Side::Right => parameters.base.foot_offset_left,
        };

        Some(Step {
            forward: swing_sole.position().x(),
            left: swing_sole.position().y() - swing_sole_base_offset.y(),
            turn: swing_sole.orientation().inner.euler_angles().2,
        })
    }
}

fn compensate_arm_motion_with_torso_tilt(
    arm_command: &ArmCommand,
    torso_tilt_compensation_factor: f32,
) -> f32 {
    (arm_command.shoulder_pitch() - FRAC_PI_2) * torso_tilt_compensation_factor
}
