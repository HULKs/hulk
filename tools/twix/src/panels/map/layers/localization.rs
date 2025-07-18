use std::sync::Arc;

use color_eyre::Result;
use eframe::{
    egui::Align2,
    epaint::{Color32, Stroke},
};

use coordinate_systems::Field;
use linear_algebra::{point, Pose2};
use types::{field_dimensions::FieldDimensions, localization::ScoredPose};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Localization {
    poses: BufferHandle<Option<Vec<ScoredPose>>>,
}

impl Layer<Field> for Localization {
    const NAME: &'static str = "Localization";

    fn new(nao: Arc<Nao>) -> Self {
        let poses = nao.subscribe_value("Control.additional_outputs.localization.pose_hypotheses");
        Self { poses }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(poses) = self.poses.get_last_value()?.flatten() {
            let circle_radius = 0.1;
            let line_length = 0.16;
            let fill_color = Color32::LIGHT_RED;
            let stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };
            for scored_pose in poses {
                let pose = Pose2::new(
                    point![scored_pose.state.mean.x, scored_pose.state.mean.y],
                    scored_pose.state.mean.z,
                );
                painter.pose(pose, circle_radius, line_length, fill_color, stroke);
                painter.floating_text(
                    pose.position(),
                    Align2::LEFT_BOTTOM,
                    format!("{:.2}", scored_pose.score),
                    Default::default(),
                    Color32::WHITE,
                );
            }
        }
        Ok(())
    }
}
