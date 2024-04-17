use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Field;
use eframe::{
    emath::Align2,
    epaint::{Color32, FontId, Stroke},
};
use linear_algebra::Point2;
use types::{field_dimensions::FieldDimensions, pose_types::PoseType};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PoseDetection {
    detected_pose_types: ValueBuffer,
}

impl Layer<Field> for PoseDetection {
    const NAME: &'static str = "Pose Positions";

    fn new(nao: Arc<Nao>) -> Self {
        Self {
            detected_pose_types: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::DetectionTop,
                output: Output::Additional {
                    path: "detected_pose_types".to_string(),
                },
            }),
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
        let detected_pose_types: Vec<(PoseType, Point2<Field>)> =
            self.detected_pose_types.require_latest()?;

        for (pose_type, position) in detected_pose_types {
            painter.circle(position, 0.15, Color32::RED, position_stroke);
            painter.text(
                position,
                Align2::CENTER_BOTTOM,
                format!("{:?}", pose_type),
                FontId::default(),
                Color32::WHITE,
            );
        }

        Ok(())
    }
}
