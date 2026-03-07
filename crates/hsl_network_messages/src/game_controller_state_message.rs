use std::{
    convert::{TryFrom, TryInto},
    ffi::c_char,
    mem::size_of,
    ptr::read,
    time::Duration,
};

use color_eyre::{Report, Result, eyre::bail};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    HULKS_TEAM_NUMBER, NONE_TEAM_NUMBER, PlayerNumber,
    bindings::{
        COMPETITION_TYPE_LARGE, COMPETITION_TYPE_MIDDLE, COMPETITION_TYPE_SMALL,
        GAME_PHASE_EXTRATIME, GAME_PHASE_NORMAL, GAME_PHASE_PENALTYSHOOT, GAME_PHASE_TIMEOUT,
        GAMECONTROLLER_STRUCT_HEADER, GAMECONTROLLER_STRUCT_VERSION, MAX_NUM_PLAYERS,
        PENALTY_BALL_HOLDING, PENALTY_ILLEGAL_POSITIONING, PENALTY_INCAPABLE_ROBOT,
        PENALTY_LEAVING_THE_FIELD, PENALTY_LOCAL_GAME_STUCK, PENALTY_MOTION_IN_SET, PENALTY_NONE,
        PENALTY_PICK_UP, PENALTY_PLAYING_WITH_ARMS_HANDS, PENALTY_PUSHING, PENALTY_SENT_OFF,
        PENALTY_SUBSTITUTE, RoboCupGameControlData, RobotInfo, SET_PLAY_CORNER_KICK,
        SET_PLAY_DIRECT_FREE_KICK, SET_PLAY_GOAL_KICK, SET_PLAY_INDIRECT_FREE_KICK, SET_PLAY_NONE,
        SET_PLAY_PENALTY_KICK, SET_PLAY_THROW_IN, STATE_FINISHED, STATE_INITIAL, STATE_PLAYING,
        STATE_READY, STATE_SET, TEAM_BLACK, TEAM_BLUE, TEAM_BROWN, TEAM_GRAY, TEAM_GREEN,
        TEAM_ORANGE, TEAM_PURPLE, TEAM_RED, TEAM_WHITE, TEAM_YELLOW,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathIntrospect)]
pub struct GameControllerStateMessage {
    pub competition_type: CompetitionType,
    pub stopped: bool, //TODO: muss nochmal mit GameController abgeglichen werden ob das so stehen bleibt
    pub game_phase: GamePhase,
    pub game_state: GameState,
    pub sub_state: Option<SubState>,
    pub half: Half,
    pub remaining_time_in_half: Duration,
    pub secondary_time: Duration,
    pub hulks_team: TeamState,
    pub opponent_team: TeamState,
    pub kicking_team: Option<Team>,
    pub hulks_team_is_home_after_coin_toss: bool,
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
            bail!("unexpected header: {:?}", message.header);
        }
        if message.version != GAMECONTROLLER_STRUCT_VERSION {
            bail!("unexpected version: {}", message.version);
        }
        let (hulks_team_index, opponent_team_index) =
            match (message.teams[0].teamNumber, message.teams[1].teamNumber) {
                (HULKS_TEAM_NUMBER, _) => (0, 1),
                (_, HULKS_TEAM_NUMBER) => (1, 0),
                _ => bail!(
                    "failed to find HULKs team, teams were {:?} and {:?}",
                    message.teams[0],
                    message.teams[1]
                ),
            };
        const MAXIMUM_NUMBER_OF_PENALTY_SHOOTS: u8 = 16;
        if message.teams[hulks_team_index].penaltyShot >= MAXIMUM_NUMBER_OF_PENALTY_SHOOTS {
            bail!(
                "unexpected penalty shoot index for team HULKs: {:?}",
                message.teams[hulks_team_index].penaltyShot
            );
        }
        if message.teams[opponent_team_index].penaltyShot >= MAXIMUM_NUMBER_OF_PENALTY_SHOOTS {
            bail!(
                "unexpected penalty shoot index for opponent team: {:?}",
                message.teams[opponent_team_index].penaltyShot
            );
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
        if message.playersPerTeam > MAX_NUM_PLAYERS {
            bail!(
                "unexpected number of players per team. Expected: {MAX_NUM_PLAYERS}. Got: {}",
                message.playersPerTeam
            );
        }
        let hulks_players = (0..MAX_NUM_PLAYERS)
            .map(|player_index| {
                message.teams[hulks_team_index].players[player_index as usize].try_into()
            })
            .collect::<Result<Vec<_>>>()?;
        let opponent_players = (0..MAX_NUM_PLAYERS)
            .map(|player_index| {
                message.teams[opponent_team_index].players[player_index as usize].try_into()
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(GameControllerStateMessage {
            competition_type: CompetitionType::try_from(message.competitionType)?,
            stopped: message.stopped != 0,
            game_phase: GamePhase::try_from(message.gamePhase, message.kickingTeam)?,
            game_state: GameState::try_from(message.state)?,
            sub_state: SubState::try_from(message.setPlay)?,
            half: message.firstHalf.try_into()?,
            remaining_time_in_half: Duration::from_secs(message.secsRemaining.max(0).try_into()?),
            secondary_time: Duration::from_secs(message.secondaryTime.max(0).try_into()?),
            hulks_team: TeamState {
                team_number: message.teams[hulks_team_index].teamNumber,
                field_player_color: message.teams[hulks_team_index]
                    .fieldPlayerColour
                    .try_into()?,
                goal_keeper_color: message.teams[hulks_team_index]
                    .goalkeeperColour
                    .try_into()?,
                goal_keeper_player_number: match message.teams[hulks_team_index].goalkeeper {
                    0 => None,
                    1 => Some(PlayerNumber::One),
                    2 => Some(PlayerNumber::Two),
                    3 => Some(PlayerNumber::Three),
                    4 => Some(PlayerNumber::Four),
                    5 => Some(PlayerNumber::Five),
                    _ => bail!(
                        "unexpected goal keeper player number {}",
                        message.teams[hulks_team_index].goalkeeper
                    ),
                },
                score: message.teams[hulks_team_index].score,
                penalty_shoot_index: message.teams[hulks_team_index].penaltyShot,
                penalty_shoots: hulks_penalty_shoots,
                remaining_amount_of_messages: message.teams[hulks_team_index].messageBudget,
                players: hulks_players,
            },
            opponent_team: TeamState {
                team_number: message.teams[opponent_team_index].teamNumber,
                field_player_color: message.teams[opponent_team_index]
                    .fieldPlayerColour
                    .try_into()?,
                goal_keeper_color: message.teams[opponent_team_index]
                    .goalkeeperColour
                    .try_into()?,
                goal_keeper_player_number: match message.teams[opponent_team_index].goalkeeper {
                    0 => None,
                    1 => Some(PlayerNumber::One),
                    2 => Some(PlayerNumber::Two),
                    3 => Some(PlayerNumber::Three),
                    4 => Some(PlayerNumber::Four),
                    5 => Some(PlayerNumber::Five),
                    _ => {
                        eprintln!(
                            "unexpected goal keeper player number {}, defaulting to PlayerNumber::One",
                            message.teams[opponent_team_index].goalkeeper
                        );
                        Some(PlayerNumber::One)
                    }
                },
                score: message.teams[opponent_team_index].score,
                penalty_shoot_index: message.teams[opponent_team_index].penaltyShot,
                penalty_shoots: opponent_penalty_shoots,
                remaining_amount_of_messages: message.teams[opponent_team_index].messageBudget,
                players: opponent_players,
            },
            kicking_team: Team::try_from(message.kickingTeam).ok(),
            hulks_team_is_home_after_coin_toss: hulks_team_index == 0,
        })
    }
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum CompetitionType {
    Small,
    Middle,
    Large,
}

impl CompetitionType {
    fn try_from(competition_type: u8) -> Result<Self> {
        match competition_type {
            COMPETITION_TYPE_SMALL => Ok(CompetitionType::Small),
            COMPETITION_TYPE_MIDDLE => Ok(CompetitionType::Middle),
            COMPETITION_TYPE_LARGE => Ok(CompetitionType::Large),
            _ => bail!("unexpected competition type"),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum GamePhase {
    #[default]
    Normal,
    PenaltyShootout {
        kicking_team: Team,
    },
    Extratime,
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
            GAME_PHASE_EXTRATIME => Ok(GamePhase::Extratime),
            GAME_PHASE_TIMEOUT => Ok(GamePhase::Timeout),
            _ => bail!("unexpected game phase"),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Team {
    Hulks,
    Opponent,
}

impl TryFrom<u8> for Team {
    type Error = Report;

    fn try_from(team_number: u8) -> Result<Self> {
        let team = if team_number == NONE_TEAM_NUMBER {
            return Err(Report::msg("kicking team is none"));
        } else if team_number == HULKS_TEAM_NUMBER {
            Team::Hulks
        } else {
            Team::Opponent
        };
        Ok(team)
    }
}

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum SubState {
    #[default]
    DirectFreeKick,
    IndirectFreeKick,
    PenaltyKick,
    ThrowIn,
    GoalKick,
    CornerKick,
}

impl SubState {
    fn try_from(sub_state: u8) -> Result<Option<Self>> {
        match sub_state {
            SET_PLAY_NONE => Ok(None),
            SET_PLAY_DIRECT_FREE_KICK => Ok(Some(SubState::DirectFreeKick)),
            SET_PLAY_INDIRECT_FREE_KICK => Ok(Some(SubState::IndirectFreeKick)),
            SET_PLAY_PENALTY_KICK => Ok(Some(SubState::PenaltyKick)),
            SET_PLAY_THROW_IN => Ok(Some(SubState::ThrowIn)),
            SET_PLAY_GOAL_KICK => Ok(Some(SubState::GoalKick)),
            SET_PLAY_CORNER_KICK => Ok(Some(SubState::CornerKick)),
            _ => bail!("unexpected sub state"),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct TeamState {
    pub team_number: u8,
    pub field_player_color: TeamColor,
    pub goal_keeper_color: TeamColor,
    pub goal_keeper_player_number: Option<PlayerNumber>,
    pub score: u8,
    pub penalty_shoot_index: u8,
    pub penalty_shoots: Vec<PenaltyShoot>,
    pub remaining_amount_of_messages: u16,
    pub players: Vec<Player>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
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
    pub warning: u8,
    pub caution: u8,
}

impl TryFrom<RobotInfo> for Player {
    type Error = Report;

    fn try_from(player: RobotInfo) -> Result<Self> {
        let remaining = Duration::from_secs(player.secsTillUnpenalised.into());
        Ok(Self {
            penalty: Penalty::try_from(remaining, player.penalty)?,
            warning: player.warnings,
            caution: player.cautions,
        })
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum Penalty {
    IllegalPosition { remaining: Duration },
    MotionInSet { remaining: Duration },
    LocalGameStuck { remaining: Duration },
    IncapableRobot { remaining: Duration },
    PickUp { remaining: Duration },
    BallHolding { remaining: Duration },
    LeavingTheField { remaining: Duration },
    PlayingWithArmsHands { remaining: Duration },
    Pushing { remaining: Duration },
    SentOff { remaining: Duration },
    Substitute { remaining: Duration },
}

impl Penalty {
    fn try_from(remaining: Duration, penalty: u8) -> Result<Option<Self>> {
        match penalty {
            PENALTY_NONE => Ok(None),
            PENALTY_ILLEGAL_POSITIONING => Ok(Some(Penalty::IllegalPosition { remaining })),
            PENALTY_MOTION_IN_SET => Ok(Some(Penalty::MotionInSet { remaining })),
            PENALTY_LOCAL_GAME_STUCK => Ok(Some(Penalty::LocalGameStuck { remaining })),
            PENALTY_INCAPABLE_ROBOT => Ok(Some(Penalty::IncapableRobot { remaining })),
            PENALTY_PICK_UP => Ok(Some(Penalty::PickUp { remaining })),
            PENALTY_BALL_HOLDING => Ok(Some(Penalty::BallHolding { remaining })),
            PENALTY_LEAVING_THE_FIELD => Ok(Some(Penalty::LeavingTheField { remaining })),
            PENALTY_PLAYING_WITH_ARMS_HANDS => {
                Ok(Some(Penalty::PlayingWithArmsHands { remaining }))
            }
            PENALTY_PUSHING => Ok(Some(Penalty::Pushing { remaining })),
            PENALTY_SENT_OFF => Ok(Some(Penalty::SentOff { remaining })),
            PENALTY_SUBSTITUTE => Ok(Some(Penalty::Substitute { remaining })),
            _ => bail!("unexpected penalty type"),
        }
    }
}

impl Default for Penalty {
    fn default() -> Self {
        Self::PickUp {
            remaining: Duration::ZERO,
        }
    }
}
