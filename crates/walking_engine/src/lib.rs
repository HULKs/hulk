use arm::ArmOverrides as _;
use coordinate_systems::{Ground, Robot, Walk};
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
    cycle_time::CycleTime, joints::body::BodyJoints, motion_command::KickVariant,
    motor_commands::MotorCommands, obstacle_avoiding_arms::ArmCommands, sensor_data::SensorData,
    step_plan::Step, support_foot::Side,
};

mod anatomic_constraints;
mod arm;
pub mod feet;
mod foot_leveling;
mod gyro_balancing;
mod kick_state;
pub mod kick_steps;
pub mod mode;
pub mod parameters;
mod step_plan;
mod step_state;
mod stiffness;

/// # WalkingEngine
/// This node generates foot positions and thus leg angles for the robot to execute a walk.
/// The algorithm to compute the feet trajectories is loosely based on the work of Bernhard Hengst
/// at the team rUNSWift. An explanation of this algorithm can be found in the team's research
/// report from 2014 (<http://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>).

pub struct Context<'a> {
    pub parameters: &'a Parameters,
    pub kick_steps: &'a KickSteps,
    pub cycle_time: &'a CycleTime,
    pub center_of_mass: &'a Point3<Robot>,
    pub sensor_data: &'a SensorData,
    pub robot_to_ground: Option<&'a Isometry3<Robot, Ground>>,
    pub gyro: nalgebra::Vector3<f32>,
    pub current_joints: BodyJoints,
    pub robot_to_walk: Isometry3<Robot, Walk>,
    pub obstacle_avoiding_arms: &'a ArmCommands,
}

pub trait WalkTransition {
    fn stand(self, context: &Context) -> Mode;
    fn walk(self, context: &Context, request: Step) -> Mode;
    fn kick(self, context: &Context, variant: KickVariant, side: Side, strength: f32) -> Mode;
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct Engine {
    pub mode: Mode,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            mode: Mode::Standing(Standing {}),
        }
    }
}

impl Engine {
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

    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.mode
            .compute_commands(context)
            .override_with_arms(context)
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
