use std::{ffi::c_char, mem::size_of, ptr::read, slice::from_raw_parts, time::Duration};

use color_eyre::{eyre::bail, Report, Result};
use coordinate_systems::{Field, Ground};
use linear_algebra::{point, Pose2};
use serde::{Deserialize, Serialize};

use crate::{
    bindings::{
        RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_STRUCT_HEADER,
        GAMECONTROLLER_RETURN_STRUCT_VERSION,
    },
    BallPosition, HULKS_TEAM_NUMBER,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct GameControllerReturnMessage {
    pub jersey_number: usize,
    pub fallen: bool,
    pub pose: Pose2<Field>,
    pub ball: Option<BallPosition<Ground>>,
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
            jersey_number: message.playerNum as usize,
            fallen: match message.fallen {
                1 => true,
                0 => false,
                _ => bail!("unexpected fallen state"),
            },
            pose: Pose2::new(
                point![message.pose[0] / 1000.0, message.pose[1] / 1000.0],
                message.pose[2],
            ),
            ball: if message.ballAge == -1.0 {
                None
            } else {
                Some(BallPosition {
                    position: point![message.ball[0] / 1000.0, message.ball[1] / 1000.0],
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
        let (ball_position, ball_age) = match &message.ball {
            Some(ball_position) => (
                [
                    ball_position.position.x() * 1000.0,
                    ball_position.position.y() * 1000.0,
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
            playerNum: message.jersey_number as u8,
            teamNum: HULKS_TEAM_NUMBER,
            fallen: u8::from(message.fallen),
            pose: [
                message.pose.position().x() * 1000.0,
                message.pose.position().y() * 1000.0,
                message.pose.orientation().angle(),
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

    use super::*;

    #[test]
    fn zero_isometry() {
        let input_message = GameControllerReturnMessage {
            jersey_number: 1,
            fallen: false,
            pose: Pose2::default(),
            ball: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0);
        assert_relative_eq!(output_message.pose[1], 0.0);
        assert_relative_eq!(output_message.pose[2], 0.0);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(input_message_again.pose, Pose2::default());
    }

    #[test]
    fn one_to_the_left_isometry() {
        let input_message = GameControllerReturnMessage {
            jersey_number: 1,
            fallen: false,
            pose: Pose2::new(point![0.0, 1.0], FRAC_PI_2),
            ball: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_2, epsilon = 0.001);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.pose,
            Pose2::new(point![0.0, 1.0], FRAC_PI_2),
            epsilon = 0.001
        );
    }

    #[test]
    fn one_schräg_to_the_top_right_isometry() {
        let input_message = GameControllerReturnMessage {
            jersey_number: 1,
            fallen: false,
            pose: Pose2::new(point![1.0, 1.0], FRAC_PI_4),
            ball: None,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(
            input_message.pose.as_transform::<Ground>() * point![1.0 / SQRT_2, -1.0 / SQRT_2],
            point![2.0, 1.0],
            epsilon = 0.001
        );
        assert_relative_eq!(
            input_message.pose.position(),
            point![1.0, 1.0],
            epsilon = 0.001
        );

        assert_relative_eq!(output_message.pose[0], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_4, epsilon = 0.001);

        let input_message_again: GameControllerReturnMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.pose,
            Pose2::new(point![1.0, 1.0], FRAC_PI_4),
            epsilon = 0.001
        );
    }
}
