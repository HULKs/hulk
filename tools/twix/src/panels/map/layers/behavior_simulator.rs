use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use nalgebra::{point, Isometry2, Point2, UnitComplex};
use types::{field_dimensions::FieldDimensions, motion_command::MotionCommand};

use crate::{
    nao::Nao, panels::map::layer::Layer, players_value_buffer::PlayersValueBuffer,
    twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

const TRANSPARENT_BLUE: Color32 = Color32::from_rgba_premultiplied(0, 0, 202, 150);
const TRANSPARENT_LIGHT_BLUE: Color32 = Color32::from_rgba_premultiplied(136, 170, 182, 150);

pub struct BehaviorSimulator {
    robot_to_field: PlayersValueBuffer,
    motion_command: PlayersValueBuffer,
    head_yaw: PlayersValueBuffer,
    ball: ValueBuffer,
}

impl Layer for BehaviorSimulator {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = PlayersValueBuffer::try_new(
            nao.clone(),
            "BehaviorSimulator.main.databases",
            "main_outputs.robot_to_field",
        )
        .unwrap();
        let motion_command = PlayersValueBuffer::try_new(
            nao.clone(),
            "BehaviorSimulator.main.databases",
            "main_outputs.motion_command",
        )
        .unwrap();
        let sensor_data = PlayersValueBuffer::try_new(
            nao.clone(),
            "BehaviorSimulator.main.databases",
            "main_outputs.sensor_data.positions.head.yaw",
        )
        .unwrap();
        let ball = nao.subscribe_output("BehaviorSimulator.main_outputs.ball.position");

        Self {
            robot_to_field,
            motion_command,
            head_yaw: sensor_data,
            ball,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        for (player_number, value_buffer) in self.robot_to_field.0.iter() {
            let Ok(robot_to_field): Result<Isometry2<f32>> = value_buffer.parse_latest() else {
                continue;
            };

            let pose_color = Color32::from_white_alpha(63);
            let pose_stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };

            if let Ok(MotionCommand::Walk { path, .. }) =
                self.motion_command.0[player_number].parse_latest()
            {
                painter.path(
                    robot_to_field,
                    path,
                    TRANSPARENT_BLUE,
                    TRANSPARENT_LIGHT_BLUE,
                    0.025,
                );
            }

            if let Ok(head_yaw) = self.head_yaw.0[player_number].parse_latest::<f32>() {
                let fov_stroke = Stroke {
                    width: 0.002,
                    color: Color32::YELLOW,
                };
                let fov_angle = 45.0_f32.to_radians();
                let fov_rotation = UnitComplex::from_angle(fov_angle / 2.0);
                let fov_range = 3.0;
                let fov_corner = point![fov_range, 0.0];
                let head_rotation = UnitComplex::from_angle(head_yaw);
                painter.line_segment(
                    robot_to_field.translation.vector.into(),
                    robot_to_field * head_rotation * fov_rotation * fov_corner,
                    fov_stroke,
                );
                painter.line_segment(
                    robot_to_field.translation.vector.into(),
                    robot_to_field * head_rotation * fov_rotation.inverse() * fov_corner,
                    fov_stroke,
                );
            }

            painter.pose(robot_to_field, 0.15, 0.25, pose_color, pose_stroke);
        }

        if let Ok(ball_position) = self.ball.parse_latest::<Point2<f32>>() {
            painter.ball(ball_position, 0.05);
        }

        Ok(())
    }
}
