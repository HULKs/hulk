use anyhow::Result;
use macros::{module, require_some};

use crate::{
    control::PathObstacle,
    framework::configuration,
    types::{FieldDimensions, MotionCommand, Role, SensorData},
};
use crate::{framework::configuration::RolePositions, types::WorldState};

use super::{
    action::Action,
    defend::{defend_goal_pose, defend_left_pose, defend_right_pose},
    dribble, fall_safely,
    head::look_for_ball,
    in_walk_kick, penalize, search, sit_down, stand, stand_up,
    support_striker::support_striker_pose,
    unstiff, walk_backwards,
    walk_behind_ball::walk_behind_ball_pose,
    walk_to_pose::walk_and_stand_with_head,
};

pub struct Behavior {}

#[module(control)]
#[input(path = world_state, data_type = WorldState)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.behavior.injected_motion_command, data_type = Option<MotionCommand>)]
#[parameter(path = control.behavior.role_positions, data_type = RolePositions)]
#[parameter(path = control.behavior.dribble_pose, data_type = configuration::DribblePose)]
#[parameter(path = control.behavior.walk_to_pose, data_type = configuration::WalkToPose)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[additional_output(path = path_obstacles, data_type = Vec<PathObstacle>)]
#[main_output(data_type = MotionCommand)]
impl Behavior {}

impl Behavior {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let world_state = require_some!(context.world_state);

        if let Some(command) = context.injected_motion_command {
            return Ok(MainOutputs {
                motion_command: Some(command.clone()),
            });
        }

        let actions = match world_state.robot.role {
            Role::DefenderLeft => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::DefendLeft,
            ],
            Role::DefenderRight => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::DefendRight,
            ],
            Role::Keeper => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::DefendGoal,
            ],
            Role::Loser => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::WalkBackwards,
            ],
            Role::ReplacementKeeper => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::DefendGoal,
            ],
            Role::Searcher => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::Search,
            ],
            Role::Striker => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::InWalkKick,
                Action::Dribble,
                Action::WalkBehindBall,
            ],
            Role::StrikerSupporter => vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::FallSafely,
                Action::StandUp,
                Action::Stand,
                Action::SupportStriker,
            ],
        };

        let motion_command = actions
            .iter()
            .find_map(|action| match action {
                Action::Unstiff => unstiff::execute(world_state),
                Action::SitDown => sit_down::execute(world_state),
                Action::Penalize => penalize::execute(world_state),
                Action::FallSafely => fall_safely::execute(world_state),
                Action::StandUp => stand_up::execute(world_state),
                Action::Stand => stand::execute(world_state),
                Action::InWalkKick => in_walk_kick::execute(world_state, context.field_dimensions),
                Action::Dribble => {
                    dribble::execute(world_state, context.field_dimensions, context.dribble_pose)
                }
                Action::WalkBackwards => walk_backwards::execute(world_state),
                Action::Search => search::execute(world_state),
                Action::WalkBehindBall => {
                    let pose = walk_behind_ball_pose(
                        world_state,
                        context.field_dimensions,
                        context.dribble_pose,
                    )?;
                    walk_and_stand_with_head(
                        pose,
                        world_state,
                        look_for_ball(world_state.ball),
                        context.field_dimensions,
                        context.walk_to_pose,
                        &mut context.path_obstacles,
                    )
                }
                Action::DefendGoal => {
                    let pose = defend_goal_pose(
                        world_state,
                        context.field_dimensions,
                        context.role_positions,
                    )?;
                    walk_and_stand_with_head(
                        pose,
                        world_state,
                        look_for_ball(world_state.ball),
                        context.field_dimensions,
                        context.walk_to_pose,
                        &mut context.path_obstacles,
                    )
                }
                Action::DefendLeft => {
                    let pose = defend_left_pose(
                        world_state,
                        context.field_dimensions,
                        context.role_positions,
                    )?;
                    walk_and_stand_with_head(
                        pose,
                        world_state,
                        look_for_ball(world_state.ball),
                        context.field_dimensions,
                        context.walk_to_pose,
                        &mut context.path_obstacles,
                    )
                }
                Action::DefendRight => {
                    let pose = defend_right_pose(
                        world_state,
                        context.field_dimensions,
                        context.role_positions,
                    )?;
                    walk_and_stand_with_head(
                        pose,
                        world_state,
                        look_for_ball(world_state.ball),
                        context.field_dimensions,
                        context.walk_to_pose,
                        &mut context.path_obstacles,
                    )
                }
                Action::SupportStriker => {
                    let pose = support_striker_pose(
                        world_state,
                        context.field_dimensions,
                        context.role_positions,
                    )?;
                    walk_and_stand_with_head(
                        pose,
                        world_state,
                        look_for_ball(world_state.ball),
                        context.field_dimensions,
                        context.walk_to_pose,
                        &mut context.path_obstacles,
                    )
                }
            })
            .unwrap_or_else(|| {
                panic!(
                    "There has to be at least one action available, world_state: {:#?}",
                    world_state
                )
            });

        Ok(MainOutputs {
            motion_command: Some(motion_command),
        })
    }
}
