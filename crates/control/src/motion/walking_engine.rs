use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Robot;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use linear_algebra::Point3;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::body::BodyJoints,
    motion_command::{ArmMotion, MotionCommand},
    motion_selection::{MotionSafeExits, MotionType},
    motor_commands::MotorCommands,
    parameters::StepPlannerParameters,
    robot_kinematics::RobotKinematics,
    sensor_data::SensorData,
    step_plan::Step,
    support_foot::Side,
    walk_command::WalkCommand,
    walking_engine::{KickStepsParameters, WalkingEngineParameters},
};

use self::{
    arms::Arm,
    mode::{
        kicking::Kicking, standing::Standing, stopping::Stopping, walking::Walking, Mode,
        WalkTransition,
    },
};

mod anatomic_constraints;
mod arms;
mod balancing;
mod catching_steps;
mod feet;
mod kicking;
mod mode;
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
    _step_planner_config: Parameter<StepPlannerParameters, "step_planner">,
    kick_steps: Parameter<KickStepsParameters, "kick_steps">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    walk_return_offset: CyclerState<Step, "walk_return_offset">,

    cycle_time: Input<CycleTime, "cycle_time">,
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    motion_command: Input<MotionCommand, "motion_command">,
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    sensor_data: Input<SensorData, "sensor_data">,
    walk_command: Input<WalkCommand, "walk_command">,
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
            left_arm: Some(Arm::default()),
            right_arm: Some(Arm::default()),
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
        let left_arm = self.left_arm.take().unwrap();
        let right_arm = self.right_arm.take().unwrap();

        self.mode = match *context.walk_command {
            WalkCommand::Stand => self.mode.stand(&context),
            WalkCommand::Walk { step } => self.mode.walk(&context, step),
            WalkCommand::Kick {
                variant,
                side,
                strength,
            } => self.mode.kick(&context, variant, side, strength),
        };

        let left_arm = match context.motion_command {
            MotionCommand::Walk {
                left_arm: ArmMotion::Swing,
                ..
            } => left_arm.swing(&context),
            MotionCommand::Walk {
                left_arm: ArmMotion::PullTight,
                ..
            } => left_arm.pull_tight(&context),
            MotionCommand::InWalkKick { .. } => left_arm,
            _ => left_arm.swing(&context),
        };
        let right_arm = match context.motion_command {
            MotionCommand::Walk {
                right_arm: ArmMotion::Swing,
                ..
            } => right_arm.swing(&context),
            MotionCommand::Walk {
                right_arm: ArmMotion::PullTight,
                ..
            } => right_arm.pull_tight(&context),
            MotionCommand::InWalkKick { .. } => right_arm,
            _ => right_arm.swing(&context),
        };

        let motor_commands =
            self.mode
                .compute_commands(&context, &left_arm, &right_arm, self.filtered_gyro.state());

        self.left_arm = Some(left_arm);
        self.right_arm = Some(right_arm);

        *context.walk_return_offset = match self.mode {
            Mode::Standing(_) => Step::ZERO,
            Mode::Starting(_) => Step::ZERO,
            Mode::Walking(Walking { step, .. })
            | Mode::Kicking(Kicking { step, .. })
            | Mode::Stopping(Stopping { step }) => {
                let feet = step.feet_at(context.cycle_time.start_time, context.parameters);
                let swing_foot_base_offset = match step.support_side {
                    Side::Left => context.parameters.base.foot_offset_right,
                    Side::Right => context.parameters.base.foot_offset_left,
                };
                Step {
                    forward: feet.swing_foot.x(),
                    left: feet.swing_foot.y() - swing_foot_base_offset.y(),
                    turn: feet.swing_turn,
                }
            }
        };

        context.motion_safe_exits[MotionType::Walk] = matches!(self.mode, Mode::Standing(..));

        Ok(MainOutputs {
            walk_motor_commands: motor_commands.into(),
        })
    }
}
