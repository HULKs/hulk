use std::f32::consts::FRAC_PI_2;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot, UpcomingSupport, Walk};
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use kinematics::forward;
use linear_algebra::{
    vector, Isometry2, Isometry3, Orientation3, Point2, Point3, Pose3, Vector2, Vector3,
};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{body::BodyJoints, Joints},
    motion_selection::{MotionSafeExits, MotionType},
    motor_commands::MotorCommands,
    obstacle_avoiding_arms::{ArmCommand, ArmCommands},
    sensor_data::SensorData,
    step::Step,
    support_foot::Side,
    walk_command::WalkCommand,
};
use walking_engine::{kick_steps::KickSteps, mode::Mode, parameters::Parameters, Context, Engine};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkingEngine {
    engine: Engine,
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
    max_step_size: Parameter<Step, "step_planner.max_step_size">,
    kick_steps: Parameter<KickSteps, "kick_steps">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    last_actuated_motor_commands:
        CyclerState<MotorCommands<Joints<f32>>, "last_actuated_motor_commands">,

    cycle_time: Input<CycleTime, "cycle_time">,
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    sensor_data: Input<SensorData, "sensor_data">,
    walk_command: Input<WalkCommand, "walk_command">,
    robot_to_ground: Input<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
    obstacle_avoiding_arms: Input<ArmCommands, "obstacle_avoiding_arms">,
    zero_moment_point: Input<Point2<Ground>, "zero_moment_point">,
    consecutive_cycles_zero_moment_point_outside_support_polygon:
        Input<i32, "consecutive_cycles_zero_moment_point_outside_support_polygon">,
    debug_output: AdditionalOutput<Engine, "walking.engine">,
    robot_to_walk: AdditionalOutput<Isometry3<Robot, Walk>, "walking.robot_to_walk">,
    walking_engine_mode: CyclerState<Mode, "walking_engine_mode">,
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
                .angular_velocity
                .inner,
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

        let step_compensation = if let WalkCommand::Walk { step } = *cycle_context.walk_command {
            let parameters = cycle_context.parameters.base.torso_tilt;
            let translational = nalgebra::vector![
                step.forward.abs() * parameters.forward,
                step.left.abs() * parameters.left,
            ]
            .norm();
            let rotational = step.turn.abs() * parameters.turn;
            translational + rotational
        } else {
            0.0
        };

        let robot_to_walk = Isometry3::from_parts(
            vector![
                cycle_context.parameters.base.torso_offset,
                0.0,
                cycle_context.parameters.base.walk_height,
            ],
            Orientation3::new(
                Vector3::y_axis()
                    * (cycle_context.parameters.base.torso_tilt_base
                        + step_compensation
                        + arm_compensation),
            ),
        );
        let imu = cycle_context.sensor_data.inertial_measurement_unit;
        let orientation =
            Orientation3::from_euler_angles(imu.roll_pitch.x(), imu.roll_pitch.y(), 0.0);

        let context = Context {
            parameters: cycle_context.parameters,
            max_step_size: cycle_context.max_step_size,
            kick_steps: cycle_context.kick_steps,
            cycle_time: cycle_context.cycle_time,
            center_of_mass: cycle_context.center_of_mass,
            force_sensitive_resistors: &cycle_context.sensor_data.force_sensitive_resistors,
            robot_orientation: &orientation,
            robot_to_ground: cycle_context.robot_to_ground,
            gyro: self.filtered_gyro.state(),
            last_actuated_joints: cycle_context.last_actuated_motor_commands.positions.into(),
            measured_joints: cycle_context.sensor_data.positions.into(),
            robot_to_walk,
            obstacle_avoiding_arms: cycle_context.obstacle_avoiding_arms,
            zero_moment_point: cycle_context.zero_moment_point,
            consecutive_cycles_zero_moment_point_outside_support_polygon: cycle_context
                .consecutive_cycles_zero_moment_point_outside_support_polygon,
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

        *cycle_context.ground_to_upcoming_support = self
            .calculate_return_offset(
                cycle_context.parameters,
                &motor_commands.positions,
                cycle_context.robot_to_ground,
            )
            .unwrap_or_default();
        cycle_context.motion_safe_exits[MotionType::Walk] = self.engine.is_standing();

        cycle_context
            .debug_output
            .fill_if_subscribed(|| self.engine.clone());
        cycle_context
            .robot_to_walk
            .fill_if_subscribed(|| robot_to_walk);
        *cycle_context.walking_engine_mode = self.engine.mode;

        Ok(MainOutputs {
            walk_motor_commands: motor_commands.into(),
        })
    }

    fn calculate_return_offset(
        &self,
        parameters: &Parameters,
        last_actuated: &BodyJoints,
        robot_to_ground: Option<&Isometry3<Robot, Ground>>,
    ) -> Option<Isometry2<Ground, UpcomingSupport>> {
        let robot_to_ground = *robot_to_ground?;
        let support_side = self.engine.mode.support_side()?;

        let left_upcoming = ground_to_upcoming_support(
            forward::left_sole_to_robot(&last_actuated.left_leg).as_pose(),
            parameters.base.foot_offset_left.xy(),
            robot_to_ground,
        );
        let right_upcoming = ground_to_upcoming_support(
            forward::right_sole_to_robot(&last_actuated.right_leg).as_pose(),
            parameters.base.foot_offset_right.xy(),
            robot_to_ground,
        );

        Some(match support_side {
            Side::Left => right_upcoming,
            Side::Right => left_upcoming,
        })
    }
}

fn ground_to_upcoming_support(
    sole: Pose3<Robot>,
    foot_offset: Vector2<Walk>,
    robot_to_ground: Isometry3<Robot, Ground>,
) -> Isometry2<Ground, UpcomingSupport> {
    let sole_in_ground = robot_to_ground * sole;

    let translation = vector![
        sole_in_ground.position().x() - foot_offset.x(),
        sole_in_ground.position().y() - foot_offset.y(),
    ];
    let yaw = -sole_in_ground
        .orientation()
        .inner
        .inverse()
        .euler_angles()
        .2;
    Isometry2::new(translation, yaw).inverse()
}

fn compensate_arm_motion_with_torso_tilt(
    arm_command: &ArmCommand,
    torso_tilt_compensation_factor: f32,
) -> f32 {
    (arm_command.shoulder_pitch() - FRAC_PI_2) * torso_tilt_compensation_factor
}
