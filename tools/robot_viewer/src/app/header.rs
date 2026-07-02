use eframe::egui::{Color32, Context, RichText, TopBottomPanel, Ui};

use crate::state::{ConnectionStatus, PoseSource, StreamState, StreamStatus, ViewerStatusSnapshot};

use super::RobotViewerApp;

impl RobotViewerApp {
    pub(super) fn header(&mut self, context: &Context, state: &ViewerStatusSnapshot) {
        TopBottomPanel::top("header")
            .min_height(86.0)
            .show(context, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(RichText::new("Robot Viewer").strong());
                        ui.separator();
                        ui.label(
                            RichText::new(format!("namespace {}", self.namespace)).monospace(),
                        );
                        ui.separator();
                        ui.label(RichText::new(format!("router {}", self.router)).monospace());
                        ui.separator();
                        connection_status(ui, &state.connection);
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        stream_status(ui, "field", &state.field_status);
                        stream_status(ui, "localization", &state.localization_status);
                        stream_status(ui, "visual odometer", &state.visual_odometer_status);
                        stream_status(ui, "kinematics", &state.robot_kinematics_status);
                        stream_status(ui, "camera matrix", &state.camera_matrix_status);
                        stream_status(
                            ui,
                            "calibrated intrinsics",
                            &state.calibrated_intrinsics_status,
                        );
                        stream_status(ui, "camera", &state.camera_status);
                        stream_status(ui, "objects", &state.objects_status);
                        stream_status(ui, "associations", &state.field_mark_associations_status);
                        ui.separator();
                        ui.label(format!("pose: {}", pose_source_label(self.pose_source)));
                        if ui
                            .button(pose_source_button_label(self.pose_source))
                            .clicked()
                        {
                            self.pose_source = match self.pose_source {
                                PoseSource::Localization => PoseSource::VisualOdometer,
                                PoseSource::VisualOdometer => PoseSource::Localization,
                            };
                        }
                    });
                });
            });
    }
}

fn pose_source_label(pose_source: PoseSource) -> &'static str {
    match pose_source {
        PoseSource::Localization => "localization latest",
        PoseSource::VisualOdometer => "visual odometer latest",
    }
}

fn pose_source_button_label(pose_source: PoseSource) -> &'static str {
    match pose_source {
        PoseSource::Localization => "use visual odometer pose",
        PoseSource::VisualOdometer => "use localization pose",
    }
}

fn connection_status(ui: &mut Ui, status: &ConnectionStatus) {
    let (text, color) = match status {
        ConnectionStatus::Starting => ("starting".to_string(), Color32::GRAY),
        ConnectionStatus::Connecting => ("connecting".to_string(), Color32::YELLOW),
        ConnectionStatus::Subscribed => ("connected".to_string(), Color32::LIGHT_GREEN),
        ConnectionStatus::Error(error) => (format!("error: {error}"), Color32::LIGHT_RED),
    };
    ui.colored_label(color, RichText::new(text).strong());
}

fn stream_status(ui: &mut Ui, name: &str, status: &StreamStatus) {
    let (label, color) = match status.state {
        StreamState::Waiting => ("waiting", Color32::GRAY),
        StreamState::Matched => ("matched", Color32::YELLOW),
        StreamState::Live => ("live", Color32::LIGHT_GREEN),
        StreamState::Empty => ("empty", Color32::YELLOW),
        StreamState::Error => ("error", Color32::LIGHT_RED),
    };
    let response = ui.colored_label(
        color,
        format!("{name}: {label} ({} pubs)", status.publisher_count),
    );
    if let Some(detail) = &status.detail {
        response.on_hover_text(detail);
    }
}
