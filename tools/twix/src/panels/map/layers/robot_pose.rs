use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Pose2;
use types::field_dimensions::FieldDimensions;

use crate::{nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct RobotPose {}

impl Layer<Ground> for RobotPose {
    const NAME: &'static str = "Robot Pose";

    fn new(_nao: Arc<Nao>) -> Self {
        Self {}
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let pose_color = Color32::from_white_alpha(127);
        let pose_stroke = Stroke {
            width: 0.02,
            color: Color32::BLACK,
        };
        painter.pose(Pose2::zero(), 0.15, 0.25, pose_color, pose_stroke);
        Ok(())
    }
}
