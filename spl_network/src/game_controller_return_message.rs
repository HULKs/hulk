use std::{mem::size_of, slice::from_raw_parts};

use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

use crate::{
    bindings::{
        RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_STRUCT_HEADER,
        GAMECONTROLLER_RETURN_STRUCT_VERSION,
    },
    BallPosition, HULKS_TEAM_NUMBER,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameControllerReturnMessage {
    pub player_number: u8,
    pub fallen: bool,
    pub robot_to_field: Isometry2<f32>,
    pub ball_position: Option<BallPosition>,
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
                GAMECONTROLLER_RETURN_STRUCT_HEADER[0] as i8,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[1] as i8,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[2] as i8,
                GAMECONTROLLER_RETURN_STRUCT_HEADER[3] as i8,
            ],
            version: GAMECONTROLLER_RETURN_STRUCT_VERSION,
            playerNum: message.player_number,
            teamNum: HULKS_TEAM_NUMBER,
            fallen: if message.fallen { 1 } else { 0 },
            pose: [
                message.robot_to_field.translation.vector.x * 1000.0,
                message.robot_to_field.translation.vector.y * 1000.0,
                message.robot_to_field.rotation.angle(),
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
    use nalgebra::{point, vector};

    use super::*;

    #[test]
    fn zero_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::default(),
            ball_position: BallPosition::Unknown,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0);
        assert_relative_eq!(output_message.pose[1], 0.0);
        assert_relative_eq!(output_message.pose[2], 0.0);
    }

    #[test]
    fn one_to_the_left_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::new(vector![0.0, 1.0], FRAC_PI_2),
            ball_position: BallPosition::Unknown,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_2, epsilon = 0.001);
    }

    #[test]
    fn one_schräg_to_the_top_right_isometry() {
        let input_message = GameControllerReturnMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::new(vector![1.0, 1.0], FRAC_PI_4),
            ball_position: BallPosition::Unknown,
        };
        let output_message: RoboCupGameControlReturnData = input_message.into();

        assert_relative_eq!(
            input_message.robot_to_field * point![1.0 / SQRT_2, -1.0 / SQRT_2],
            point![2.0, 1.0],
            epsilon = 0.001
        );
        assert_relative_eq!(
            input_message.robot_to_field * point![0.0, 0.0],
            point![1.0, 1.0],
            epsilon = 0.001
        );

        assert_relative_eq!(output_message.pose[0], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_4, epsilon = 0.001);
    }
}
