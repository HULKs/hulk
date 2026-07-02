use std::sync::Arc;

use color_eyre::Result;
use eframe::{
    egui::Align2,
    epaint::{Color32, Stroke},
};

use coordinate_systems::Field;
use linear_algebra::{Pose2, point};
use ros_z_debug::{SampleRecord, TopicObservation};
use types::{field_dimensions::FieldDimensions, localization::ScoredPose};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct Localization {
    poses: TopicObservation<Vec<ScoredPose>>,
}

impl Layer<Field> for Localization {
    const NAME: &'static str = "Localization";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let poses = backend
            .observer()
            .observe_typed("localization/pose_hypotheses")
            .expect("failed to construct pose hypotheses observer")
            .spawn();

        Self { poses }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(SampleRecord { value: poses, .. }) = self.poses.latest().as_deref() {
            let circle_radius = 0.1;
            let line_length = 0.16;
            let fill_color = Color32::LIGHT_RED;
            let covariance_fill_color =
                Color32::from_rgba_unmultiplied(fill_color.r(), fill_color.g(), fill_color.b(), 40);
            let stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };
            for scored_pose in poses {
                let pose = Pose2::new(
                    point![scored_pose.state.mean.x, scored_pose.state.mean.y],
                    scored_pose.state.mean.z,
                );
                let covariance = scored_pose
                    .state
                    .covariance
                    .fixed_view::<2, 2>(0, 0)
                    .into_owned();
                painter.covariance(pose.position(), covariance, stroke, covariance_fill_color);
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
