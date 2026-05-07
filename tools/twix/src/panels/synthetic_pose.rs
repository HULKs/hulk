use std::sync::Arc;

use eframe::egui::{Color32, Pos2, Response, Sense, Stroke, Ui, Vec2, Widget};
use ros_z::MessageTypeInfo;
use serde::{Deserialize, Serialize};

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    value_buffer::BufferHandle,
};

#[derive(Debug, Clone, Serialize, Deserialize, MessageTypeInfo)]
#[ros_msg(type_name = "twix_demo/msg/RobotPose")]
struct RobotPose {
    x: f64,
    y: f64,
    theta: f64,
    confidence: f64,
    state: String,
}

impl ros_z::msg::ZMessage for RobotPose {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}

pub struct SyntheticPosePanel {
    pose_buffer: BufferHandle<RobotPose>,
    _robot: Arc<Robot>,
}

impl<'a> Panel<'a> for SyntheticPosePanel {
    const NAME: &'static str = "Synthetic Pose";

    fn new(context: PanelCreationContext) -> Self {
        Self {
            pose_buffer: context
                .robot
                .subscribe_topic_value("/twix_demo/robot_pose", std::time::Duration::ZERO),
            _robot: context.robot,
        }
    }
}

impl Widget for &mut SyntheticPosePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let available = ui.available_size_before_wrap();
        let desired_size = Vec2::new(available.x.max(240.0), available.y.max(240.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_stroke(
            rect,
            8.0,
            Stroke::new(1.0, Color32::DARK_GRAY),
            eframe::egui::StrokeKind::Inside,
        );

        let center = rect.center();
        painter.line_segment(
            [
                Pos2::new(rect.left(), center.y),
                Pos2::new(rect.right(), center.y),
            ],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        painter.line_segment(
            [
                Pos2::new(center.x, rect.top()),
                Pos2::new(center.x, rect.bottom()),
            ],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );

        match self.pose_buffer.get_last_value() {
            Ok(Some(pose)) => {
                let scale = 40.0;
                let robot_center = Pos2::new(
                    center.x + pose.x as f32 * scale,
                    center.y - pose.y as f32 * scale,
                );
                let heading = Vec2::angled(-(pose.theta as f32)) * 24.0;
                let confidence_color =
                    Color32::from_rgb((pose.confidence.clamp(0.0, 1.0) * 255.0) as u8, 180, 120);

                painter.circle_filled(robot_center, 10.0, confidence_color);
                painter.line_segment(
                    [robot_center, robot_center + heading],
                    Stroke::new(3.0, Color32::WHITE),
                );
                painter.text(
                    rect.left_top() + Vec2::new(8.0, 8.0),
                    eframe::egui::Align2::LEFT_TOP,
                    format!(
                        "state={} confidence={:.2}\npos=({:.2}, {:.2}) theta={:.2}",
                        pose.state, pose.confidence, pose.x, pose.y, pose.theta
                    ),
                    eframe::egui::FontId::monospace(13.0),
                    Color32::WHITE,
                );
            }
            Ok(None) => {
                painter.text(
                    rect.center(),
                    eframe::egui::Align2::CENTER_CENTER,
                    "waiting for /twix_demo/robot_pose",
                    eframe::egui::FontId::proportional(16.0),
                    Color32::GRAY,
                );
            }
            Err(error) => {
                painter.text(
                    rect.center(),
                    eframe::egui::Align2::CENTER_CENTER,
                    error.to_string(),
                    eframe::egui::FontId::proportional(14.0),
                    Color32::LIGHT_RED,
                );
            }
        }

        response
    }
}
