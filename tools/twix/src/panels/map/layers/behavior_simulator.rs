use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Field, Ground};
use linear_algebra::{IntoFramed, Isometry2, Point2};
use types::{field_dimensions::FieldDimensions, motion_command::MotionCommand};

use crate::{nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter};

const TRANSPARENT_BLUE: Color32 = Color32::from_rgba_premultiplied(0, 0, 202, 150);
const TRANSPARENT_LIGHT_BLUE: Color32 = Color32::from_rgba_premultiplied(136, 170, 182, 150);

pub struct BehaviorSimulator {
    ground_to_field: PlayersValueBuffer,
    motion_command: PlayersValueBuffer,
    head_yaw: PlayersValueBuffer,
    ball: ValueBuffer,
}

impl Layer<Field> for BehaviorSimulator {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field = PlayersValueBuffer::try_new(
            nao.clone(),
            "BehaviorSimulator.main.databases",
            "main_outputs.ground_to_field",
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
        let ball = nao.subscribe_value("BehaviorSimulator.main_outputs.ball.position");
        Self {
            ground_to_field,
            motion_command,
            head_yaw: sensor_data,
            ball,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        for (player_number, value_buffer) in self.ground_to_field.0.iter() {
            let Ok(ground_to_field): Result<Isometry2<Ground, Field>> = value_buffer.parse_latest()
            else {
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
                let ground_painter = painter.transform_painter(ground_to_field.inverse());
                ground_painter.path(path, TRANSPARENT_BLUE, TRANSPARENT_LIGHT_BLUE, 0.025);
            }

            if let Ok(head_yaw) = self.head_yaw.0[player_number].parse_latest::<f32>() {
                let fov_stroke = Stroke {
                    width: 0.002,
                    color: Color32::YELLOW,
                };
                let fov_angle = 45.0_f32.to_radians();
                let fov_rotation = nalgebra::UnitComplex::from_angle(fov_angle / 2.0);
                let fov_range = 3.0;
                let fov_corner = nalgebra::point![fov_range, 0.0];
                let head_rotation = nalgebra::UnitComplex::from_angle(head_yaw);
                painter.line_segment(
                    ground_to_field * Point2::origin(),
                    (ground_to_field.inner * head_rotation * fov_rotation * fov_corner).framed(),
                    fov_stroke,
                );
                painter.line_segment(
                    ground_to_field.translation(),
                    (ground_to_field.inner * head_rotation * fov_rotation.inverse() * fov_corner)
                        .framed(),
                    fov_stroke,
                );
            }

            painter.pose(
                ground_to_field.as_pose(),
                0.15,
                0.25,
                pose_color,
                pose_stroke,
            );
        }

        if let Ok(ball_position) = self.ball.parse_latest::<Point2<Field>>() {
            painter.ball(ball_position, 0.05);
        }

        Ok(())
    }
}
