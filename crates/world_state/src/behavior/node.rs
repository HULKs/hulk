use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

use crate::{
    behavior::new_behavior::behavior_tree::{Status},
    selection, sequence, condition, action,
};

#[derive(Deserialize, Serialize)]
pub struct Behavior {}

pub struct CaptainBlackboard {
    pub world_state: WorldState,
    pub output: Option<MotionCommand>,
}
#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    world_state: Input<WorldState, "world_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let mut blackboard = CaptainBlackboard {
            world_state: context.world_state.clone(),
            output: None,
        };

        let tree = selection!(
            sequence!(
                condition!(is_primary_state, PrimaryState::Playing),
                condition!(has_ball_position),
                action!(walk_to_ball)
            ),
            action!(stand)
        );
        let status = tree.tick(&mut blackboard);

        let motion_command: MotionCommand = match status {
            Status::Success | Status::Running => {
                blackboard.output.take().unwrap_or(MotionCommand::Stand {
                    head: HeadMotion::Center {
                        image_region_target: ImageRegion::Center,
                    },
                })
            }
            Status::Failure => MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            },
        };

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}

fn is_primary_state(context: &mut CaptainBlackboard, primary_state: PrimaryState) -> bool {
    context.world_state.robot.primary_state == primary_state
}

fn has_ball_position(context: &mut CaptainBlackboard) -> bool {
    context.world_state.ball.is_some()
}

fn stand(context: &mut CaptainBlackboard) -> Status {
    context.output = Some(MotionCommand::Stand {
        head: HeadMotion::LookAround,
    });
    Status::Success
}

fn walk_to_ball(context: &mut CaptainBlackboard) -> Status {
    if let Some(ball) = &context.world_state.ball {
        context.output = Some(MotionCommand::WalkWithVelocity {
            head: HeadMotion::LookAt {
                target: ball.ball_in_ground,
                image_region_target: ImageRegion::Top,
            },
            velocity: ball.ball_in_ground.coords(),
            angular_velocity: 0.0,
        });
        Status::Success
    } else {
        Status::Failure
    }
}
