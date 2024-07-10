use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::{
    emath::Align2,
    epaint::{Color32, FontId, Stroke},
};
use types::{field_dimensions::FieldDimensions, pose_kinds::PoseKindPosition};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct PoseDetection {
    accepted_pose_kind_positions: BufferHandle<Option<Vec<PoseKindPosition<Field>>>>,
    rejected_pose_kind_positions: BufferHandle<Option<Vec<PoseKindPosition<Field>>>>,
    referee_pose_kind_position: BufferHandle<Option<Option<PoseKindPosition<Field>>>>,
}

impl Layer<Field> for PoseDetection {
    const NAME: &'static str = "Pose Positions";

    fn new(nao: Arc<Nao>) -> Self {
        let accepted_pose_kind_positions = nao
            .subscribe_value("ObjectDetectionTop.additional_outputs.accepted_pose_kind_positions");
        let rejected_pose_kind_positions = nao
            .subscribe_value("ObjectDetectionTop.additional_outputs.rejected_pose_kind_positions");
        let referee_pose_kind_position =
            nao.subscribe_value("ObjectDetectionTop.additional_outputs.referee_pose_kind_position");
        Self {
            accepted_pose_kind_positions,
            rejected_pose_kind_positions,
            referee_pose_kind_position,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(accepted_pose_kind_positions) = self
            .accepted_pose_kind_positions
            .get_last_value()?
            .flatten()
        else {
            return Ok(());
        };
        let Some(rejected_pose_kind_positions) = self
            .rejected_pose_kind_positions
            .get_last_value()?
            .flatten()
        else {
            return Ok(());
        };
        let Some(referee_pose_kind_position) =
            self.referee_pose_kind_position.get_last_value()?.flatten()
        else {
            return Ok(());
        };

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
