use arm::{Arm, ArmOverrides as _};
use coordinate_systems::{Ground, Robot};
use kick_steps::KickSteps;
use linear_algebra::{Isometry3, Point3};
use mode::{
    catching::Catching, kicking::Kicking, standing::Standing, starting::Starting,
    stopping::Stopping, walking::Walking, Mode,
};
use parameters::Parameters;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    cycle_time::CycleTime,
    joints::body::BodyJoints,
    motion_command::{ArmMotion, KickVariant},
    motor_commands::MotorCommands,
    sensor_data::SensorData,
    step_plan::Step,
    support_foot::Side,
};

mod anatomic_constraints;
mod arm;
mod feet;
mod foot_leveling;
mod gyro_balancing;
mod kick_state;
pub mod kick_steps;
mod mode;
pub mod parameters;
mod step_plan;
mod step_state;
mod stiffness;

pub struct Context<'a> {
    pub parameters: &'a Parameters,
    pub kick_steps: &'a KickSteps,
    pub cycle_time: &'a CycleTime,
    pub center_of_mass: &'a Point3<Robot>,
    pub sensor_data: &'a SensorData,
    pub robot_to_ground: Option<&'a Isometry3<Robot, Ground>>,
    pub gyro: nalgebra::Vector3<f32>,
    pub current_joints: BodyJoints,
}

pub trait WalkTransition {
    fn stand(self, context: &Context) -> Mode;
    fn walk(self, context: &Context, request: Step) -> Mode;
    fn kick(self, context: &Context, variant: KickVariant, side: Side, strength: f32) -> Mode;
}

/// # WalkingEngine
/// This node generates foot positions and thus leg angles for the robot to execute a walk.
/// The algorithm to compute the feet trajectories is loosely based on the work of Bernhard Hengst
/// at the team rUNSWift. An explanation of this algorithm can be found in the team's research
/// report from 2014 (<http://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>).
#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct Engine {
    pub mode: Mode,
    pub left_arm: Option<Arm>,
    pub right_arm: Option<Arm>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            mode: Mode::Standing(Standing {}),
            left_arm: Some(Arm::default()),
            right_arm: Some(Arm::default()),
        }
    }

    pub fn stand(&mut self, context: &Context) {
        self.mode = self.mode.stand(context);
    }

    pub fn walk(&mut self, context: &Context, request: Step) {
        self.mode = self.mode.walk(context, request);
    }

    pub fn kick(&mut self, context: &Context, variant: KickVariant, side: Side, strength: f32) {
        self.mode = self.mode.kick(context, variant, side, strength);
    }

    pub fn tick(&mut self, context: &Context) {
        self.mode.tick(context);
    }

    pub fn transition_arm(&mut self, context: &Context, side: Side, motion: ArmMotion) {
        let container = match side {
            Side::Left => &mut self.left_arm,
            Side::Right => &mut self.right_arm,
        };
        // enter the functional world...
        let arm = container.take().unwrap();
        let arm = match motion {
            ArmMotion::Swing => arm.swing(context),
            ArmMotion::PullTight => arm.pull_tight(context),
        };
        // do not forget to put it back ;)
        *container = Some(arm);
    }

    pub fn compute_commands(
        &self,
        parameters: &Parameters,
        kick_steps: &KickSteps,
    ) -> MotorCommands<BodyJoints> {
        self.mode
            .compute_commands(parameters, kick_steps)
            .override_with_arms(
                &parameters.swinging_arms,
                self.left_arm.as_ref().unwrap(),
                self.right_arm.as_ref().unwrap(),
            )
    }

    pub fn is_standing(&self) -> bool {
        matches!(self.mode, Mode::Standing(_))
    }

    pub fn support_side(&self) -> Option<Side> {
        match self.mode {
            Mode::Standing(_) => None,
            Mode::Starting(Starting { step })
            | Mode::Walking(Walking { step, .. })
            | Mode::Kicking(Kicking { step, .. })
            | Mode::Stopping(Stopping { step, .. })
            | Mode::Catching(Catching { step, .. }) => Some(step.plan.support_side),
        }
    }
}
