use std::{ffi::c_char, mem::size_of, ptr::read, slice::from_raw_parts, time::Duration};

use color_eyre::{eyre::bail, Report, Result};
use nalgebra::{point, vector, Isometry2};
use serde::{Deserialize, Serialize};

use crate::{
    bindings::{
        RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_STRUCT_HEADER,
        GAMECONTROLLER_RETURN_STRUCT_VERSION,
    },
    BallPosition, PlayerNumber, HULKS_TEAM_NUMBER,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct GameControllerReturnMessage {
    pub player_number: PlayerNumber,
    pub fallen: bool,
    pub ground_to_field: Isometry2<f32>,
    pub ball_position: Option<BallPosition>,
}

impl TryFrom<&[u8]> for GameControllerReturnMessage {
    type Error = Report;

    fn try_from(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < size_of::<RoboCupGameControlReturnData>() {
            bail!("buffer too small");
        }
        let message = unsafe { read(buffer.as_ptr() as *const RoboCupGameControlReturnData) };
        message.try_into()
    }
}

impl TryFrom<RoboCupGameControlReturnData> for GameControllerReturnMessage {
    type Error = Report;

    fn try_from(message: RoboCupGameControlReturnData) -> Result<Self> {
        if message.header[0] != GAMECONTROLLER_RETURN_STRUCT_HEADER[0] as c_char
            && message.header[1] != GAMECONTROLLER_RETURN_STRUCT_HEADER[1] as c_char
            && message.header[2] != GAMECONTROLLER_RETURN_STRUCT_HEADER[2] as c_char
            && message.header[3] != GAMECONTROLLER_RETURN_STRUCT_HEADER[3] as c_char
        {
            bail!("unexpected header");
        }
        if message.version != GAMECONTROLLER_RETURN_STRUCT_VERSION {
            bail!("unexpected version");
        }
        if message.teamNum != HULKS_TEAM_NUMBER {
            bail!("unexpected team number != {}", HULKS_TEAM_NUMBER);
        }
        Ok(Self {
            player_number: match message.playerNum {
                1 => PlayerNumber::One,
                2 => PlayerNumber::Two,
                3 => PlayerNumber::Three,
                4 => PlayerNumber::Four,
                5 => PlayerNumber::Five,
                6 => PlayerNumber::Six,
                7 => PlayerNumber::Seven,
                _ => bail!("unexpected player number {}", message.playerNum),
            },
            fallen: match message.fallen {
                1 => true,
                0 => false,
                _ => bail!("unexpected fallen state"),
            },
            ground_to_field: Isometry2::new(
                vector![message.pose[0] / 1000.0, message.pose[1] / 1000.0],
                message.pose[2],
            ),
            ball_position: if message.ballAge == -1.0 {
                None
            } else {
                Some(BallPosition {
                    relative_position: point![message.ball[0] / 1000.0, message.ball[1] / 1000.0],
                    age: Duration::from_secs_f32(message.ballAge),
                })
            },
        })
    }
}

impl From<GameControllerReturnMessage> for Vec<u8> {
    fn from(message: GameControllerReturnMessage) -> Self {
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

impl From<GameControllerReturnMessage> for RoboCupGameControlReturnData {
    fn from(message: GameControllerReturnMessage) -> Self {
        let (ball_position, ball_age) = match &message.ball_position {
            Some(ball_position) => (
                [
                    ball_position.relative_position.x * 1000.0,
                    ball_position.relative_position.y * 1000.0,
                ],
                ball_position.age.as_secs_f32(),
            ),
            None => ([0.0; 2], -1.0),
        };
        RoboCupGameControlReturnData {
            header: [
                GAMECONTROLLER_RETURN_STRUCT_HEADER[0] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[1] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[2] as c_char,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[3] as c_char,
            ],
            version: GAMECONTROLLER_RETURN_STRUCT_VERSION,
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
            fallen: u8::from(message.fallen),
            pose: [
                message.ground_to_field.translation.vector.x * 1000.0,
                message.ground_to_field.translation.vector.y * 1000.0,
                message.ground_to_field.rotation.angle(),
            ],
            ballAge: ball_age,
            ball: ball_position,
        }
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, SQRT_2};

    use approx::assert_relative_eq;
    use nalgebra::vector;

    use super::*;

    #[test]
    fn zero_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: PlayerNumber::One,
            fallen: false,
            ground_to_field: Isometry2::default(),
            ball_position: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0);
        assert_relative_eq!(output_message.pose[1], 0.0);
        assert_relative_eq!(output_message.pose[2], 0.0);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(input_message_again.ground_to_field, Isometry2::default());
    }

    #[test]
    fn one_to_the_left_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: PlayerNumber::One,
            fallen: false,
            ground_to_field: Isometry2::new(vector![0.0, 1.0], FRAC_PI_2),
            ball_position: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_2, epsilon = 0.001);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.ground_to_field,
            Isometry2::new(vector![0.0, 1.0], FRAC_PI_2),
            epsilon = 0.001
        );
    }

    #[test]
    fn one_schräg_to_the_top_right_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: PlayerNumber::One,
            fallen: false,
            ground_to_field: Isometry2::new(vector![1.0, 1.0], FRAC_PI_4),
            ball_position: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(
            input_message.ground_to_field * point![1.0 / SQRT_2, -1.0 / SQRT_2],
            point![2.0, 1.0],
            epsilon = 0.001
        );
        assert_relative_eq!(
            input_message.ground_to_field * point![0.0, 0.0],
            point![1.0, 1.0],
            epsilon = 0.001
        );

        assert_relative_eq!(output_message.pose[0], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_4, epsilon = 0.001);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.ground_to_field,
            Isometry2::new(vector![1.0, 1.0], FRAC_PI_4),
            epsilon = 0.001
        );
    }
}
