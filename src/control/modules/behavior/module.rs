use anyhow::Result;
use module_derive::{module, require_some};
use spl_network::Team;
use types::{
    FieldDimensions, FilteredGameState, MotionCommand, PathObstacle, Role, SensorData, WorldState,
};

use crate::framework::configuration;

use super::{
    action::Action,
    defend::Defend,
    dribble, fall_safely, in_walk_kick, penalize, search, sit_down, stand, stand_up,
    support_striker, unstiff, walk_backwards, walk_to_kick_off,
    walk_to_pose::{WalkAndStand, WalkPathPlanner},
};

pub struct Behavior {}

#[module(control)]
#[input(path = world_state, data_type = WorldState)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.behavior, data_type = configuration::Behavior)]
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

        if let Some(command) = &context.behavior.injected_motion_command {
            return Ok(MainOutputs {
                motion_command: Some(command.clone()),
            });
        }

        let mut actions = vec![
            Action::Unstiff,
            Action::SitDown,
            Action::Penalize,
            Action::FallSafely,
            Action::StandUp,
            Action::Stand,
        ];

        match world_state.robot.role {
            Role::DefenderLeft => actions.push(Action::DefendLeft),
            Role::DefenderRight => actions.push(Action::DefendRight),
            Role::Keeper => actions.push(Action::DefendGoal),
            Role::Loser => actions.push(Action::WalkBackwards),
            Role::ReplacementKeeper => actions.push(Action::DefendGoal),
            Role::Searcher => actions.push(Action::Search),
            Role::Striker => match world_state.filtered_game_state {
                None | Some(FilteredGameState::Playing { ball_is_free: true }) => {
                    actions.push(Action::InWalkKick);
                    actions.push(Action::Dribble);
                }
                Some(FilteredGameState::Ready {
                    kicking_team: Team::Hulks,
                }) => {
                    actions.push(Action::WalkToKickOff);
                }
                _ => {
                    actions.push(Action::DefendKickOff);
                }
            },
            Role::StrikerSupporter => {
                actions.push(Action::SupportStriker);
            }
        };

        let walk_path_planner = WalkPathPlanner::new(
            world_state,
            context.field_dimensions,
            &context.behavior.path_planning,
        );
        let walk_and_stand = WalkAndStand::new(
            world_state,
            &context.behavior.walk_and_stand,
            &walk_path_planner,
        );
        let defend = Defend::new(
            world_state,
            context.field_dimensions,
            &context.behavior.role_positions,
            &walk_and_stand,
        );

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
                Action::Dribble => dribble::execute(
                    world_state,
                    context.field_dimensions,
                    &context.behavior.dribble_pose,
                    &walk_path_planner,
                    &mut context.path_obstacles,
                ),
                Action::WalkBackwards => walk_backwards::execute(world_state),
                Action::Search => search::execute(world_state),
                Action::DefendGoal => defend.goal(&mut context.path_obstacles),
                Action::DefendLeft => defend.left(&mut context.path_obstacles),
                Action::DefendRight => defend.right(&mut context.path_obstacles),
                Action::SupportStriker => support_striker::execute(
                    world_state,
                    context.field_dimensions,
                    &context.behavior.role_positions,
                    &walk_and_stand,
                    &mut context.path_obstacles,
                ),
                Action::DefendKickOff => defend.kick_off(&mut context.path_obstacles),
                Action::WalkToKickOff => walk_to_kick_off::execute(
                    world_state,
                    context.field_dimensions,
                    &context.behavior.dribble_pose,
                    &walk_and_stand,
                    &mut context.path_obstacles,
                ),
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
