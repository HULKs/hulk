use color_eyre::eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use linear_algebra::Point2;
use serde::{Deserialize, Serialize};

use framework::{MainOutput, PerceptionInput};
use types::{ball_position::BallPosition, messages::IncomingMessage};

#[derive(Deserialize, Serialize)]
pub struct TeamBallReceiver {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition<Field>>>,
    pub network_robot_obstacles: MainOutput<Vec<Point2<Ground>>>,
}

impl TeamBallReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        todo!()
    }

    pub fn cycle(&self, _context: CycleContext) -> Result<MainOutputs> {
        // if let Some(game_controller_state) = filtered_game_controller_state {
        //     match game_controller_state.game_phase {
        //         GamePhase::PenaltyShootout {
        //             kicking_team: Team::Hulks,
        //         } => return (Role::Striker, false, None),
        //         GamePhase::PenaltyShootout {
        //             kicking_team: Team::Opponent,
        //         } => return (Role::Keeper, false, None),
        //         _ => {}
        //     };
        //     if let Some(SubState::PenaltyKick) = game_controller_state.sub_state {
        //         return (current_role, false, None);
        //     }
        // }
        // if primary_state != PrimaryState::Playing {
        //     match detected_own_team_ball {
        //         None => return (current_role, false, team_ball),
        //         Some(own_team_ball) => return (current_role, false, own_team_ball),
        //     }
        todo!()
    }
}
