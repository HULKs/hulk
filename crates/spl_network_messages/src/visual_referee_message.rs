use std::{ffi::c_char, time::Duration, slice::from_raw_parts, mem::size_of};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    bindings::{
        RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_STRUCT_HEADER,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_CORNER_KICK_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_CORNER_KICK_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_FULL_TIME,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_KICK_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_KICK_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_KICK_IN_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_KICK_IN_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_PUSHING_FREE_KICK_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_PUSHING_FREE_KICK_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_SUBSTITUTION_BLUE_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_SUBSTITUTION_RED_TEAM,
        GAMECONTROLLER_RETURN_STRUCT_VRC_VERSION,
    },
    PlayerNumber, HULKS_TEAM_NUMBER,
};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy,
)]
#[repr(u8)]
pub enum VisualRefereeDecision {
    #[default]
    KickInBlueTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_KICK_IN_BLUE_TEAM,
    KickInRedTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_KICK_IN_RED_TEAM,
    GoalKickBlueTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_KICK_BLUE_TEAM,
    GoalKickRedTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_KICK_RED_TEAM,
    CornerKickBlueTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_CORNER_KICK_BLUE_TEAM,
    CornerKickRedTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_CORNER_KICK_RED_TEAM,
    GoalBlueTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_BLUE_TEAM,
    GoalRedTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_GOAL_RED_TEAM,
    PushingFreeKickBlueTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_PUSHING_FREE_KICK_BLUE_TEAM,
    PushingFreeKickRedTeam = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_PUSHING_FREE_KICK_RED_TEAM,
    FullTime = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_FULL_TIME,
    SubstitutionBlue = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_SUBSTITUTION_BLUE_TEAM,
    SubstitutionRed = GAMECONTROLLER_RETURN_STRUCT_VRC_GESTURE_SUBSTITUTION_RED_TEAM,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct VisualRefereeMessage {
    pub player_number: PlayerNumber,
    pub gesture: VisualRefereeDecision,
    pub whistle_age: Duration,
}

impl From<VisualRefereeMessage> for Vec<u8> {
    fn from(message: VisualRefereeMessage) -> Self {
        let message = message.into();
        unsafe {
            from_raw_parts(
                &message as *const RoboCupGameControlReturnData as *const u8,
                size_of::<RoboCupGameControlReturnData>(),
            )
        }
        .to_vec()
    }
}

impl From<VisualRefereeMessage> for RoboCupGameControlReturnData {
    fn from(message: VisualRefereeMessage) -> Self {
        let (ball_position, ball_age) = ([0.0; 2], message.whistle_age.as_secs_f32());
        RoboCupGameControlReturnData {
            header: [
                GAMECONTROLLER_RETURN_STRUCT_HEADER[0] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[1] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[2] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[3] as c_char,
            ],
            version: GAMECONTROLLER_RETURN_STRUCT_VRC_VERSION,
            playerNum: match message.player_number {
                PlayerNumber::One => 1,
                PlayerNumber::Two => 2,
                PlayerNumber::Three => 3,
                PlayerNumber::Four => 4,
                PlayerNumber::Five => 5,
                PlayerNumber::Six => 6,
                PlayerNumber::Seven => 7,
            },
            teamNum: HULKS_TEAM_NUMBER,
            fallen: message.gesture as u8,
            pose: Default::default(),
            ballAge: ball_age,
            ball: ball_position,
        }
    }
}
