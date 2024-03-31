use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
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

pub struct HumanPoseDetection {
    detected_human_pose_types: ValueBuffer,
}

impl Layer for HumanPoseDetection {
    const NAME: &'static str = "Detected Human Poses";

    fn new(nao: Arc<Nao>) -> Self {
        let detected_human_pose_types = nao.subscribe_output(
            CyclerOutput::from_str("DetectionTop.main_outputs.detected_pose_types").unwrap(),
        );
        Self {
            detected_human_pose_types,
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
        let detected_human_pose_types: Vec<(PoseType, Point2<Field>)> =
            self.detected_human_pose_types.require_latest()?;

        for (pose_type, position) in detected_human_pose_types {
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
