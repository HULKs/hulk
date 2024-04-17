use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, kick_step::KickSteps, motion_command::KickVariant,
    motor_commands::MotorCommands, step_plan::Step, support_foot::Side,
    walking_engine::WalkingEngineParameters,
};

use self::{
    catching::Catching, kicking::Kicking, standing::Standing, starting::Starting,
    stopping::Stopping, walking::Walking,
};

use super::CycleContext;

pub mod catching;
pub mod kicking;
pub mod standing;
pub mod starting;
pub mod stopping;
pub mod walking;

pub trait WalkTransition {
    fn stand(self, context: &CycleContext, joints: &BodyJoints) -> Mode;
    fn walk(self, context: &CycleContext, joints: &BodyJoints, request: Step) -> Mode;
    fn kick(
        self,
        context: &CycleContext,
        joints: &BodyJoints,
        variant: KickVariant,
        side: Side,
        strength: f32,
    ) -> Mode;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum Mode {
    Standing(Standing),
    Starting(Starting),
    Walking(Walking),
    Kicking(Kicking),
    Stopping(Stopping),
    Catching(Catching),
}

impl WalkTransition for Mode {
    fn stand(self, context: &CycleContext, joints: &BodyJoints) -> Mode {
        match self {
            Self::Standing(standing) => standing.stand(context, joints),
            Self::Starting(starting) => starting.stand(context, joints),
            Self::Walking(walking) => walking.stand(context, joints),
            Self::Kicking(kicking) => kicking.stand(context, joints),
            Self::Stopping(stopping) => stopping.stand(context, joints),
            Self::Catching(catching) => catching.stand(context, joints),
        }
    }

    fn walk(self, context: &CycleContext, joints: &BodyJoints, step: Step) -> Mode {
        match self {
            Self::Standing(standing) => standing.walk(context, joints, step),
            Self::Starting(starting) => starting.walk(context, joints, step),
            Self::Walking(walking) => walking.walk(context, joints, step),
            Self::Kicking(kicking) => kicking.walk(context, joints, step),
            Self::Stopping(stopping) => stopping.walk(context, joints, step),
            Self::Catching(catching) => catching.walk(context, joints, step),
        }
    }

    fn kick(
        self,
        context: &CycleContext,
        joints: &BodyJoints,
        variant: KickVariant,
        side: Side,
        strength: f32,
    ) -> Mode {
        match self {
            Self::Standing(standing) => standing.kick(context, joints, variant, side, strength),
            Self::Starting(starting) => starting.kick(context, joints, variant, side, strength),
            Self::Walking(walking) => walking.kick(context, joints, variant, side, strength),
            Self::Kicking(kicking) => kicking.kick(context, joints, variant, side, strength),
            Self::Stopping(stopping) => stopping.kick(context, joints, variant, side, strength),
            Self::Catching(catching) => catching.kick(context, joints, variant, side, strength),
        }
    }
}

impl Mode {
    pub fn compute_commands(
        &self,
        parameters: &WalkingEngineParameters,
        kick_steps: &KickSteps,
    ) -> MotorCommands<BodyJoints> {
        match self {
            Self::Standing(standing) => standing.compute_commands(parameters),
            Self::Starting(starting) => starting.compute_commands(parameters),
            Self::Walking(walking) => walking.compute_commands(parameters),
            Self::Kicking(kicking) => kicking.compute_commands(parameters, kick_steps),
            Self::Stopping(stopping) => stopping.compute_commands(parameters),
            Self::Catching(catching) => catching.compute_commands(parameters),
        }
    }

    pub fn tick(&mut self, context: &mut CycleContext, gyro: nalgebra::Vector3<f32>) {
        match self {
            Mode::Standing(standing) => standing.tick(context, gyro),
            Mode::Starting(starting) => starting.tick(context, gyro),
            Mode::Walking(walking) => walking.tick(context, gyro),
            Mode::Kicking(kicking) => kicking.tick(context, gyro),
            Mode::Stopping(stopping) => stopping.tick(context, gyro),
            Mode::Catching(catching) => catching.tick(context, gyro),
        }
    }
}
