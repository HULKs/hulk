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
    detected_pose_kinds: BufferHandle<Option<Vec<PoseKindPosition<Field>>>>,
}

impl Layer<Field> for PoseDetection {
    const NAME: &'static str = "Pose Positions";

    fn new(nao: Arc<Nao>) -> Self {
        let detected_pose_kinds =
            nao.subscribe_value("ObjectDetectionTop.additional_outputs.detected_pose_kinds");
        Self {
            detected_pose_kinds,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let position_stroke = Stroke {
            width: 0.10,
            color: Color32::BLACK,
        };
        let Some(detected_pose_kinds) = self.detected_pose_kinds.get_last_value()?.flatten() else {
            return Ok(());
        };

        for pose_kind_position in detected_pose_kinds {
            painter.circle(
                pose_kind_position.position,
                0.15,
                Color32::RED,
                position_stroke,
            );
            painter.floating_text(
                pose_kind_position.position,
                Align2::CENTER_BOTTOM,
                format!("{:?}", pose_kind_position.pose_kind),
                FontId::default(),
                Color32::WHITE,
            );
        }

        Ok(())
    }
}
