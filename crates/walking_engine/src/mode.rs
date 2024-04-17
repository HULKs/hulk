use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

use crate::{kick_steps::KickSteps, parameters::Parameters, Context, WalkTransition};

use self::{
    catching::Catching, kicking::Kicking, standing::Standing, starting::Starting,
    stopping::Stopping, walking::Walking,
};

pub mod catching;
pub mod kicking;
pub mod standing;
pub mod starting;
pub mod stopping;
pub mod walking;

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
    fn stand(self, context: &Context) -> Mode {
        match self {
            Self::Standing(standing) => standing.stand(context),
            Self::Starting(starting) => starting.stand(context),
            Self::Walking(walking) => walking.stand(context),
            Self::Kicking(kicking) => kicking.stand(context),
            Self::Stopping(stopping) => stopping.stand(context),
            Self::Catching(catching) => catching.stand(context),
        }
    }

    fn walk(self, context: &Context, step: Step) -> Mode {
        match self {
            Self::Standing(standing) => standing.walk(context, step),
            Self::Starting(starting) => starting.walk(context, step),
            Self::Walking(walking) => walking.walk(context, step),
            Self::Kicking(kicking) => kicking.walk(context, step),
            Self::Stopping(stopping) => stopping.walk(context, step),
            Self::Catching(catching) => catching.walk(context, step),
        }
    }

    fn kick(self, context: &Context, variant: KickVariant, side: Side, strength: f32) -> Mode {
        match self {
            Self::Standing(standing) => standing.kick(context, variant, side, strength),
            Self::Starting(starting) => starting.kick(context, variant, side, strength),
            Self::Walking(walking) => walking.kick(context, variant, side, strength),
            Self::Kicking(kicking) => kicking.kick(context, variant, side, strength),
            Self::Stopping(stopping) => stopping.kick(context, variant, side, strength),
            Self::Catching(catching) => catching.kick(context, variant, side, strength),
        }
    }
}

impl Mode {
    pub fn compute_commands(
        &self,
        parameters: &Parameters,
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

    pub fn tick(&mut self, context: &Context) {
        match self {
            Mode::Standing(standing) => standing.tick(context),
            Mode::Starting(starting) => starting.tick(context),
            Mode::Walking(walking) => walking.tick(context),
            Mode::Kicking(kicking) => kicking.tick(context),
            Mode::Stopping(stopping) => stopping.tick(context),
            Mode::Catching(catching) => catching.tick(context),
        }
    }
}
