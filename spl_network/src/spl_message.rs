use std::{
    convert::{TryFrom, TryInto},
    mem::size_of,
    ptr::read,
    slice::from_raw_parts,
    time::Duration,
};

use anyhow::bail;
use byteorder::{ByteOrder, NativeEndian};
use nalgebra::{point, vector, Isometry2};
use serde::{Deserialize, Serialize};

use crate::{
    bindings::{
        SPLStandardMessage, SPL_STANDARD_MESSAGE_DATA_SIZE, SPL_STANDARD_MESSAGE_STRUCT_HEADER,
        SPL_STANDARD_MESSAGE_STRUCT_VERSION,
    },
    BallPosition, HULKS_TEAM_NUMBER,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SplMessage {
    pub player_number: u8,
    pub fallen: bool,
    pub robot_to_field: Isometry2<f32>,
    pub ball_position: Option<BallPosition>,
}

impl TryFrom<&[u8]> for SplMessage {
    type Error = anyhow::Error;

    fn try_from(buffer: &[u8]) -> anyhow::Result<Self> {
        let buffer_offset_of_user_data =
            size_of::<SPLStandardMessage>() - (SPL_STANDARD_MESSAGE_DATA_SIZE as usize);
        if buffer.len() < buffer_offset_of_user_data {
            bail!("Buffer too small");
        }
        let number_of_bytes_of_user_data = NativeEndian::read_u16(
            &buffer[buffer_offset_of_user_data - 2..buffer_offset_of_user_data],
        );
        let additional_number_of_bytes_in_message = buffer.len() - buffer_offset_of_user_data;
        if number_of_bytes_of_user_data as usize != additional_number_of_bytes_in_message {
            bail!("Buffer size mismatch: numOfDataBytes != length of message remainder");
        }
        let message = unsafe { read(buffer.as_ptr() as *const SPLStandardMessage) };
        message.try_into()
    }
}

impl TryFrom<SPLStandardMessage> for SplMessage {
    type Error = anyhow::Error;

    fn try_from(message: SPLStandardMessage) -> anyhow::Result<Self> {
        if message.header[0] != SPL_STANDARD_MESSAGE_STRUCT_HEADER[0] as i8
            && message.header[1] != SPL_STANDARD_MESSAGE_STRUCT_HEADER[1] as i8
            && message.header[2] != SPL_STANDARD_MESSAGE_STRUCT_HEADER[2] as i8
            && message.header[3] != SPL_STANDARD_MESSAGE_STRUCT_HEADER[3] as i8
        {
            bail!("Unexpected header");
        }
        if message.version != SPL_STANDARD_MESSAGE_STRUCT_VERSION {
            bail!("Unexpected version");
        }
        if message.teamNum != HULKS_TEAM_NUMBER {
            bail!("Unexpected team number != {}", HULKS_TEAM_NUMBER);
        }
        Ok(Self {
            player_number: message.playerNum,
            fallen: match message.fallen {
                1 => true,
                0 => false,
                _ => bail!("Unexpected fallen state"),
            },
            robot_to_field: Isometry2::new(
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

impl From<SplMessage> for Vec<u8> {
    fn from(message: SplMessage) -> Self {
        let message: SPLStandardMessage = message.into();
        unsafe {
            from_raw_parts(
                &message as *const SPLStandardMessage as *const u8,
                size_of::<SPLStandardMessage>() - (SPL_STANDARD_MESSAGE_DATA_SIZE as usize)
                    + (message.numOfDataBytes as usize),
            )
        }
        .to_vec()
    }
}

impl From<SplMessage> for SPLStandardMessage {
    fn from(message: SplMessage) -> Self {
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
        Self {
            header: [
                SPL_STANDARD_MESSAGE_STRUCT_HEADER[0] as i8,
                SPL_STANDARD_MESSAGE_STRUCT_HEADER[1] as i8,
                SPL_STANDARD_MESSAGE_STRUCT_HEADER[2] as i8,
                SPL_STANDARD_MESSAGE_STRUCT_HEADER[3] as i8,
            ],
            version: SPL_STANDARD_MESSAGE_STRUCT_VERSION,
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
            numOfDataBytes: 0,
            data: [0; SPL_STANDARD_MESSAGE_DATA_SIZE as usize],
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
        let input_message = SplMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::default(),
            ball_position: BallPosition::Unknown,
        };
        let output_message: SPLStandardMessage = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0);
        assert_relative_eq!(output_message.pose[1], 0.0);
        assert_relative_eq!(output_message.pose[2], 0.0);

        let input_message_again: SplMessage = output_message.try_into().unwrap();

        assert_relative_eq!(input_message_again.robot_to_field, Isometry2::default());
    }

    #[test]
    fn one_to_the_left_isometry() {
        let input_message = SplMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::new(vector![0.0, 1.0], FRAC_PI_2),
            ball_position: BallPosition::Unknown,
        };
        let output_message: SPLStandardMessage = input_message.into();

        assert_relative_eq!(output_message.pose[0], 0.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[1], 1000.0, epsilon = 0.001);
        assert_relative_eq!(output_message.pose[2], FRAC_PI_2, epsilon = 0.001);

        let input_message_again: SplMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.robot_to_field,
            Isometry2::new(vector![0.0, 1.0], FRAC_PI_2),
            epsilon = 0.001
        );
    }

    #[test]
    fn one_schräg_to_the_top_right_isometry() {
        let input_message = SplMessage {
            player_number: 1,
            fallen: false,
            robot_to_field: Isometry2::new(vector![1.0, 1.0], FRAC_PI_4),
            ball_position: BallPosition::Unknown,
        };
        let output_message: SPLStandardMessage = input_message.into();

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

        let input_message_again: SplMessage = output_message.try_into().unwrap();

        assert_relative_eq!(
            input_message_again.robot_to_field,
            Isometry2::new(vector![1.0, 1.0], FRAC_PI_4),
            epsilon = 0.001
        );
    }
}
