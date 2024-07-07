use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Field;
use eframe::{
    emath::Align2,
    epaint::{Color32, FontId, Stroke},
};
use types::{field_dimensions::FieldDimensions, pose_kinds::PoseKindPosition};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PoseDetection {
    rejected_pose_kind_positions: ValueBuffer,
    accepted_pose_kind_positions: ValueBuffer,
    referee_pose_kind_position: ValueBuffer,
}

impl Layer<Field> for PoseDetection {
    const NAME: &'static str = "Pose Positions";

    fn new(nao: Arc<Nao>) -> Self {
        Self {
            rejected_pose_kind_positions: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Additional {
                    path: "rejected_pose_kind_positions".to_string(),
                },
            }),
            accepted_pose_kind_positions: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Additional {
                    path: "accepted_pose_kind_positions".to_string(),
                },
            }),
            referee_pose_kind_position: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Additional {
                    path: "referee_pose_kind_position".to_string(),
                },
            }),
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let rejected_pose_kind_positions: Vec<PoseKindPosition<Field>> =
            self.rejected_pose_kind_positions.parse_latest()?;
        let accepted_pose_kind_positions: Vec<PoseKindPosition<Field>> =
            self.accepted_pose_kind_positions.parse_latest()?;
        let referee_pose_kind_position: Option<PoseKindPosition<Field>> =
            self.referee_pose_kind_position.parse_latest()?;

        for pose_kind_position in rejected_pose_kind_positions {
            draw_pose_kind_position(painter, pose_kind_position, Color32::RED)?;
        }
        for pose_kind_position in accepted_pose_kind_positions {
            draw_pose_kind_position(painter, pose_kind_position, Color32::BLUE)?;
        }

        if let Some(referee_pose_kind_position) = referee_pose_kind_position {
            draw_pose_kind_position(painter, referee_pose_kind_position, Color32::YELLOW)?;
        }

        Ok(())
    }
}

fn draw_pose_kind_position(
    painter: &TwixPainter<Field>,
    pose_kind_position: PoseKindPosition<Field>,
    circle_color: Color32,
) -> Result<()> {
    painter.circle(
        pose_kind_position.position,
        0.15,
        circle_color,
        Stroke::new(0.10, Color32::BLACK),
    );
    painter.floating_text(
        pose_kind_position.position,
        Align2::CENTER_BOTTOM,
        format!("{:?}", pose_kind_position.pose_kind),
        FontId::default(),
        Color32::WHITE,
    );

    Ok(())
}
