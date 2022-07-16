use std::{
    convert::{TryFrom, TryInto},
    ffi::c_char,
    mem::size_of,
    ptr::read,
    time::Duration,
};

use color_eyre::{eyre::bail, Report, Result};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    bindings::{
        RoboCupGameControlData, RobotInfo, GAMECONTROLLER_STRUCT_HEADER,
        GAMECONTROLLER_STRUCT_VERSION, GAME_PHASE_NORMAL, GAME_PHASE_OVERTIME,
        GAME_PHASE_PENALTYSHOOT, GAME_PHASE_TIMEOUT, MAX_NUM_PLAYERS, PENALTY_MANUAL, PENALTY_NONE,
        PENALTY_SPL_ILLEGAL_BALL_CONTACT, PENALTY_SPL_ILLEGAL_MOTION_IN_SET,
        PENALTY_SPL_ILLEGAL_POSITION, PENALTY_SPL_ILLEGAL_POSITION_IN_SET,
        PENALTY_SPL_INACTIVE_PLAYER, PENALTY_SPL_LEAVING_THE_FIELD, PENALTY_SPL_LOCAL_GAME_STUCK,
        PENALTY_SPL_PLAYER_PUSHING, PENALTY_SPL_REQUEST_FOR_PICKUP, PENALTY_SUBSTITUTE,
        SET_PLAY_CORNER_KICK, SET_PLAY_GOAL_KICK, SET_PLAY_KICK_IN, SET_PLAY_NONE,
        SET_PLAY_PENALTY_KICK, SET_PLAY_PUSHING_FREE_KICK, STATE_FINISHED, STATE_INITIAL,
        STATE_PLAYING, STATE_READY, STATE_SET, TEAM_BLACK, TEAM_BLUE, TEAM_BROWN, TEAM_GRAY,
        TEAM_GREEN, TEAM_ORANGE, TEAM_PURPLE, TEAM_RED, TEAM_WHITE, TEAM_YELLOW,
    },
    HULKS_TEAM_NUMBER,
};

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct GameControllerStateMessage {
    pub game_phase: GamePhase,
    pub game_state: GameState,
    pub set_play: Option<SetPlay>,
    pub half: Half,
    pub remaining_time_in_half: Duration,
    pub secondary_time: Duration,
    pub hulks_team: TeamState,
    pub opponent_team: TeamState,
    pub kicking_team: Team,
}

impl TryFrom<&[u8]> for GameControllerStateMessage {
    type Error = Report;

    fn try_from(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < size_of::<RoboCupGameControlData>() {
            bail!("buffer too small");
        }
        let message = unsafe { read(buffer.as_ptr() as *const RoboCupGameControlData) };
        message.try_into()
    }
}

impl TryFrom<RoboCupGameControlData> for GameControllerStateMessage {
    type Error = Report;

    fn try_from(message: RoboCupGameControlData) -> Result<Self> {
        if message.header[0] != GAMECONTROLLER_STRUCT_HEADER[0] as c_char
            && message.header[1] != GAMECONTROLLER_STRUCT_HEADER[1] as c_char
            && message.header[2] != GAMECONTROLLER_STRUCT_HEADER[2] as c_char
            && message.header[3] != GAMECONTROLLER_STRUCT_HEADER[3] as c_char
        {
            bail!("unexpected header");
        }
        if message.version != GAMECONTROLLER_STRUCT_VERSION {
            bail!("unexpected version");
        }
        let (hulks_team_index, opponent_team_index) =
            match (message.teams[0].teamNumber, message.teams[1].teamNumber) {
                (HULKS_TEAM_NUMBER, _) => (0, 1),
                (_, HULKS_TEAM_NUMBER) => (1, 0),
                _ => bail!("failed to find HULKs team"),
            };
        const MAXIMUM_NUMBER_OF_PENALTY_SHOOTS: u8 = 16;
        if message.teams[hulks_team_index].penaltyShot >= MAXIMUM_NUMBER_OF_PENALTY_SHOOTS {
            bail!("unexpected penalty shoot index for team HULKs");
        }
        if message.teams[opponent_team_index].penaltyShot >= MAXIMUM_NUMBER_OF_PENALTY_SHOOTS {
            bail!("unexpected penalty shoot index for opponent team");
        }
        let hulks_penalty_shoots = (0..message.teams[hulks_team_index].penaltyShot)
            .map(|shoot_index| {
                if message.teams[hulks_team_index].singleShots & (1 << shoot_index) != 0 {
                    PenaltyShoot::Successful
                } else {
                    PenaltyShoot::Unsuccessful
                }
            })
            .collect();
        let opponent_penalty_shoots = (0..message.teams[opponent_team_index].penaltyShot)
            .map(|shoot_index| {
                if message.teams[opponent_team_index].singleShots & (1 << shoot_index) != 0 {
                    PenaltyShoot::Successful
                } else {
                    PenaltyShoot::Unsuccessful
                }
            })
            .collect();
        if message.playersPerTeam >= MAX_NUM_PLAYERS {
            bail!("unexpected number of players per team");
        }
        let hulks_players = (0..message.playersPerTeam)
            .map(|player_index| {
                message.teams[hulks_team_index].players[player_index as usize].try_into()
            })
            .collect::<Result<Vec<_>>>()?;
        let opponent_players = (0..message.playersPerTeam)
            .map(|player_index| {
                message.teams[opponent_team_index].players[player_index as usize].try_into()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(GameControllerStateMessage {
            game_phase: GamePhase::try_from(message.gamePhase, message.kickingTeam)?,
            game_state: GameState::try_from(message.state)?,
            set_play: SetPlay::try_from(message.setPlay)?,
            half: message.firstHalf.try_into()?,
            remaining_time_in_half: Duration::from_secs(message.secsRemaining.max(0).try_into()?),
            secondary_time: Duration::from_secs(message.secondaryTime.max(0).try_into()?),
            hulks_team: TeamState {
                team_number: message.teams[hulks_team_index].teamNumber,
                color: message.teams[hulks_team_index].teamColour.try_into()?,
                score: message.teams[hulks_team_index].score,
                penalty_shoot_index: message.teams[hulks_team_index].penaltyShot,
                penalty_shoots: hulks_penalty_shoots,
                remaining_amount_of_messages: message.teams[hulks_team_index].messageBudget,
                players: hulks_players,
            },
            opponent_team: TeamState {
                team_number: message.teams[opponent_team_index].teamNumber,
                color: message.teams[opponent_team_index].teamColour.try_into()?,
                score: message.teams[opponent_team_index].score,
                penalty_shoot_index: message.teams[opponent_team_index].penaltyShot,
                penalty_shoots: opponent_penalty_shoots,
                remaining_amount_of_messages: message.teams[opponent_team_index].messageBudget,
                players: opponent_players,
            },
            kicking_team: Team::try_from(message.kickingTeam)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum GamePhase {
    Normal,
    PenaltyShootout { kicking_team: Team },
    Overtime,
    Timeout,
}

impl GamePhase {
    fn try_from(game_phase: u8, kicking_team: u8) -> Result<Self> {
        let team = if kicking_team == HULKS_TEAM_NUMBER {
            Team::Hulks
        } else {
            Team::Opponent
        };
        match game_phase {
            GAME_PHASE_NORMAL => Ok(GamePhase::Normal),
            GAME_PHASE_PENALTYSHOOT => Ok(GamePhase::PenaltyShootout { kicking_team: team }),
            GAME_PHASE_OVERTIME => Ok(GamePhase::Overtime),
            GAME_PHASE_TIMEOUT => Ok(GamePhase::Timeout),
            _ => bail!("unexpected game phase"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum GameState {
    Initial,
    Ready,
    Set,
    Playing,
    Finished,
}

impl GameState {
    fn try_from(game_state: u8) -> Result<Self> {
        match game_state {
            STATE_INITIAL => Ok(GameState::Initial),
            STATE_READY => Ok(GameState::Ready),
            STATE_SET => Ok(GameState::Set),
            STATE_PLAYING => Ok(GameState::Playing),
            STATE_FINISHED => Ok(GameState::Finished),
            _ => bail!("unexpected game state"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, SerializeHierarchy)]
pub enum Team {
    Hulks,
    Opponent,
    Uncertain,
}

impl Default for Team {
    fn default() -> Self {
        Team::Uncertain
    }
}

impl Team {
    fn try_from(team_number: u8) -> Result<Self> {
        let team = if team_number == HULKS_TEAM_NUMBER {
            Team::Hulks
        } else {
            Team::Opponent
        };
        Ok(team)
    }
}

impl SetPlay {
    fn try_from(set_play: u8) -> Result<Option<Self>> {
        match set_play {
            SET_PLAY_NONE => Ok(None),
            SET_PLAY_GOAL_KICK => Ok(Some(SetPlay::GoalKick)),
            SET_PLAY_PUSHING_FREE_KICK => Ok(Some(SetPlay::PushingFreeKick)),
            SET_PLAY_CORNER_KICK => Ok(Some(SetPlay::CornerKick)),
            SET_PLAY_KICK_IN => Ok(Some(SetPlay::KickIn)),
            SET_PLAY_PENALTY_KICK => Ok(Some(SetPlay::PenaltyKick)),
            _ => bail!("unexpected set play"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum SetPlay {
    GoalKick,
    PushingFreeKick,
    CornerKick,
    KickIn,
    PenaltyKick,
}

impl Default for SetPlay {
    fn default() -> Self {
        SetPlay::GoalKick
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum Half {
    First,
    Second,
}

impl TryFrom<u8> for Half {
    type Error = Report;

    fn try_from(half: u8) -> Result<Self> {
        match half {
            1 => Ok(Half::First),
            0 => Ok(Half::Second),
            _ => bail!("unexpected half"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct TeamState {
    pub team_number: u8,
    pub color: TeamColor,
    pub score: u8,
    pub penalty_shoot_index: u8,
    pub penalty_shoots: Vec<PenaltyShoot>,
    pub remaining_amount_of_messages: u16,
    pub players: Vec<Player>,
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum TeamColor {
    Blue,
    Red,
    Yellow,
    Black,
    White,
    Green,
    Orange,
    Purple,
    Brown,
    Gray,
}

impl TryFrom<u8> for TeamColor {
    type Error = Report;

    fn try_from(team_color: u8) -> Result<Self> {
        match team_color {
            TEAM_BLUE => Ok(TeamColor::Blue),
            TEAM_RED => Ok(TeamColor::Red),
            TEAM_YELLOW => Ok(TeamColor::Yellow),
            TEAM_BLACK => Ok(TeamColor::Black),
            TEAM_WHITE => Ok(TeamColor::White),
            TEAM_GREEN => Ok(TeamColor::Green),
            TEAM_ORANGE => Ok(TeamColor::Orange),
            TEAM_PURPLE => Ok(TeamColor::Purple),
            TEAM_BROWN => Ok(TeamColor::Brown),
            TEAM_GRAY => Ok(TeamColor::Gray),
            _ => bail!("unexpected team color"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PenaltyShoot {
    Successful,
    Unsuccessful,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Player {
    pub penalty: Option<Penalty>,
}

impl TryFrom<RobotInfo> for Player {
    type Error = Report;

    fn try_from(player: RobotInfo) -> Result<Self> {
        let remaining = Duration::from_secs(player.secsTillUnpenalised.try_into()?);
        Ok(Self {
            penalty: Penalty::try_from(remaining, player.penalty)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum Penalty {
    IllegalBallContact { remaining: Duration },
    PlayerPushing { remaining: Duration },
    IllegalMotionInSet { remaining: Duration },
    InactivePlayer { remaining: Duration },
    IllegalPosition { remaining: Duration },
    LeavingTheField { remaining: Duration },
    RequestForPickup { remaining: Duration },
    LocalGameStuck { remaining: Duration },
    IllegalPositionInSet { remaining: Duration },
    Substitute { remaining: Duration },
    Manual { remaining: Duration },
}

impl Penalty {
    fn try_from(remaining: Duration, penalty: u8) -> Result<Option<Self>> {
        match penalty {
            PENALTY_NONE => Ok(None),
            PENALTY_SPL_ILLEGAL_BALL_CONTACT => Ok(Some(Penalty::IllegalBallContact { remaining })),
            PENALTY_SPL_PLAYER_PUSHING => Ok(Some(Penalty::PlayerPushing { remaining })),
            PENALTY_SPL_ILLEGAL_MOTION_IN_SET => {
                Ok(Some(Penalty::IllegalMotionInSet { remaining }))
            }
            PENALTY_SPL_INACTIVE_PLAYER => Ok(Some(Penalty::InactivePlayer { remaining })),
            PENALTY_SPL_ILLEGAL_POSITION => Ok(Some(Penalty::IllegalPosition { remaining })),
            PENALTY_SPL_LEAVING_THE_FIELD => Ok(Some(Penalty::LeavingTheField { remaining })),
            PENALTY_SPL_REQUEST_FOR_PICKUP => Ok(Some(Penalty::RequestForPickup { remaining })),
            PENALTY_SPL_LOCAL_GAME_STUCK => Ok(Some(Penalty::LocalGameStuck { remaining })),
            PENALTY_SPL_ILLEGAL_POSITION_IN_SET => {
                Ok(Some(Penalty::IllegalPositionInSet { remaining }))
            }
            PENALTY_SUBSTITUTE => Ok(Some(Penalty::Substitute { remaining })),
            PENALTY_MANUAL => Ok(Some(Penalty::Manual { remaining })),
            _ => bail!("unexpected penalty type"),
        }
    }
}
