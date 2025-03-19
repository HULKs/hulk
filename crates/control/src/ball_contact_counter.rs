use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use serde_json::Value;
use spl_network_messages::HulkMessage;
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, messages::IncomingMessage,
    motion_command::MotionCommand, players::Players,
};

#[derive(Deserialize, Serialize)]
pub struct BallContactCounter {
    state: StateMachine,
    own_contact_count: usize,
    other_players_had_contact: Players<bool>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    own_ball_contact_count: CyclerState<usize, "own_ball_contact_count">,
    other_striker_had_ball_contact: CyclerState<bool, "other_striker_had_ball_contact">,

    motion_command: Input<MotionCommand, "motion_command">,
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    filtered_messages: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

    x: AdditionalOutput<Value, "x">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub x: MainOutput<Value>,
}

impl BallContactCounter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            state: StateMachine::Start,
            own_contact_count: 0,
            other_players_had_contact: Players::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.state = match self.state {
            StateMachine::Start => {
                if context
                    .ball_position
                    .is_some_and(|ball| ball.position.coords().norm() < 0.3)
                {
                    StateMachine::BallWasClose
                } else {
                    StateMachine::Start
                }
            }
            StateMachine::BallWasClose => {
                if matches!(context.motion_command, MotionCommand::InWalkKick { .. }) {
                    StateMachine::Kicked
                } else {
                    StateMachine::BallWasClose
                }
            }
            StateMachine::Kicked => {
                if context
                    .ball_position
                    .is_some_and(|ball| ball.position.coords().norm() > 0.5)
                {
                    self.own_contact_count += 1;
                    StateMachine::Start
                } else {
                    StateMachine::Kicked
                }
            }
        };

        for message in context
            .filtered_messages
            .persistent
            .values()
            .flatten()
            .filter_map(|message| *message)
        {
            if let IncomingMessage::Spl(HulkMessage::Striker(striker_message)) = message {
                if striker_message.number_of_ball_contacts > 0 {
                    self.other_players_had_contact[striker_message.player_number] = true;
                }
            }
        }

        if let Some(state) = context.filtered_game_controller_state {
            if state.game_state == FilteredGameState::Set || state.sub_state.is_some() {
                self.other_players_had_contact = Players::default();
                self.own_contact_count = 0;
                self.state = StateMachine::Start;
            }
        }

        *context.other_striker_had_ball_contact = self
            .other_players_had_contact
            .iter()
            .any(|(_, had_contact)| *had_contact);
        *context.own_ball_contact_count = self.own_contact_count;

        context
            .x
            .fill_if_subscribed(|| serde_json::to_value(&self).unwrap());

        Ok(MainOutputs {
            x: serde_json::to_value(&self).unwrap().into(),
        })
    }
}

#[derive(Copy, Clone, Deserialize, Serialize)]
enum StateMachine {
    Start,
    BallWasClose,
    Kicked,
}
