use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use kinematics::forward;
use linear_algebra::{Isometry3, Point3};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::body::BodyJoints,
    kick_step::KickSteps,
    motion_command::{ArmMotion, MotionCommand},
    motion_selection::{MotionSafeExits, MotionType},
    motor_commands::MotorCommands,
    sensor_data::SensorData,
    step_plan::Step,
    support_foot::Side,
    walk_command::WalkCommand,
    walking_engine::WalkingEngineParameters,
};

use self::{
    arms::{Arm, ArmOverrides as _},
    mode::{
        catching::Catching, kicking::Kicking, standing::Standing, starting::Starting,
        stopping::Stopping, walking::Walking, Mode, WalkTransition,
    },
};

mod anatomic_constraints;
mod arms;
mod feet;
mod foot_leveling;
mod gyro_balancing;
mod kicking;
mod mode;
mod step_plan;
mod step_state;
mod stiffness;

/// # WalkingEngine
/// This node generates foot positions and thus leg angles for the robot to execute a walk.
/// The algorithm to compute the feet trajectories is loosely based on the work of Bernhard Hengst
/// at the team rUNSWift. An explanation of this algorithm can be found in the team's research
/// report from 2014 (<http://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkingEngine {
    mode: Mode,
    left_arm: Option<Arm>,
    right_arm: Option<Arm>,
    last_actuated_joints: BodyJoints,
    filtered_gyro: LowPassFilter<nalgebra::Vector3<f32>>,
}

#[context]
pub struct CreationContext {
    parameters: Parameter<WalkingEngineParameters, "walking_engine">,
}

#[context]
#[derive(Debug)]
pub struct CycleContext {
    parameters: Parameter<WalkingEngineParameters, "walking_engine">,
    kick_steps: Parameter<KickSteps, "kick_steps">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    walk_return_offset: CyclerState<Step, "walk_return_offset">,

    cycle_time: Input<CycleTime, "cycle_time">,
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    motion_command: Input<MotionCommand, "motion_command">,
    sensor_data: Input<SensorData, "sensor_data">,
    walk_command: Input<WalkCommand, "walk_command">,
    robot_to_ground: Input<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_motor_commands: MainOutput<MotorCommands<BodyJoints<f32>>>,
}

impl WalkingEngine {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            mode: Mode::Standing(Standing {}),
            left_arm: Some(Default::default()),
            right_arm: Some(Default::default()),
            last_actuated_joints: Default::default(),
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                nalgebra::Vector3::zeros(),
                context.parameters.gyro_balancing.low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.filtered_gyro.update(
            context
                .sensor_data
                .inertial_measurement_unit
                .angular_velocity,
        );

        self.mode = match *context.walk_command {
            WalkCommand::Stand => self.mode.stand(&context, &self.last_actuated_joints),
            WalkCommand::Walk { step } => {
                self.mode.walk(&context, &self.last_actuated_joints, step)
            }
            WalkCommand::Kick {
                variant,
                side,
                strength,
            } => self.mode.kick(
                &context,
                &self.last_actuated_joints,
                variant,
                side,
                strength,
            ),
        };

        self.mode.tick(&context, self.filtered_gyro.state());

        // enter the functional world...
        let left_arm = self.left_arm.take().unwrap();
        let right_arm = self.right_arm.take().unwrap();

        let left_arm = match context.motion_command.arm_motion(Side::Left) {
            Some(ArmMotion::Swing) | None => left_arm.swing(&context),
            Some(ArmMotion::PullTight) => left_arm.pull_tight(&context),
        };

        let right_arm = match context.motion_command.arm_motion(Side::Right) {
            Some(ArmMotion::Swing) | None => right_arm.swing(&context),
            Some(ArmMotion::PullTight) => right_arm.pull_tight(&context),
        };

        let motor_commands = self
            .mode
            .compute_commands(context.parameters, context.kick_steps)
            .override_with_arms(&context.parameters.swinging_arms, &left_arm, &right_arm);

        self.last_actuated_joints = motor_commands.positions;

        // do not forget to put it back ;)
        self.left_arm = Some(left_arm);
        self.right_arm = Some(right_arm);

        *context.walk_return_offset = self.calculate_return_offset(context.parameters);

        context.motion_safe_exits[MotionType::Walk] = matches!(self.mode, Mode::Standing(..));

        Ok(MainOutputs {
            walk_motor_commands: motor_commands.into(),
        })
    }

    fn calculate_return_offset(&self, parameters: &WalkingEngineParameters) -> Step {
        let left_sole = forward::left_sole_to_robot(&self.last_actuated_joints.left_leg).as_pose();
        let right_sole =
            forward::right_sole_to_robot(&self.last_actuated_joints.right_leg).as_pose();
        let support_side = match self.mode {
            Mode::Standing(_) => Side::Left,
            Mode::Starting(Starting { step })
            | Mode::Walking(Walking { step, .. })
            | Mode::Kicking(Kicking { step, .. })
            | Mode::Stopping(Stopping { step })
            | Mode::Catching(Catching { step, .. }) => step.plan.support_side,
        };
        let swing_sole = match support_side {
            Side::Left => right_sole,
            Side::Right => left_sole,
        };
        let swing_sole_base_offset = match support_side {
            Side::Left => parameters.base.foot_offset_right,
            Side::Right => parameters.base.foot_offset_left,
        };

        Step {
            forward: swing_sole.position().x(),
            left: swing_sole.position().y() - swing_sole_base_offset.y(),
            turn: swing_sole.orientation().inner.euler_angles().2,
        }
    }
}
