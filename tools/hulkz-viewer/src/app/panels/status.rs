use eframe::egui::{self, Color32};

use crate::app::format_timestamp;

use super::{Panel, ViewerApp};

pub(super) struct StatusPanel;

impl Panel for StatusPanel {
    type State = ();

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, _state: &mut Self::State) {
        let total_records = app.global_timeline.len();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Ready: {}", app.ready));
            ui.separator();
            ui.label(format!("Streams: {}", app.stream_states.len()));
            ui.separator();
            ui.label(format!("Timeline points: {total_records}"));
            ui.separator();
            ui.label(format!("Follow live: {}", app.follow_live));
            if let Some(anchor) = app.current_anchor_nanos() {
                ui.separator();
                ui.label(format!("Anchor: {}", format_timestamp(anchor)));
            }

            if let Some(backend_stats) = &app.backend_stats {
                ui.separator();
                ui.label(format!("Active sources: {}", backend_stats.active_sources));
                ui.separator();
                ui.label(format!("Queue depth: {}", backend_stats.writer_queue_depth));
            }

            if let Some(error) = &app.last_error {
                ui.separator();
                ui.colored_label(Color32::RED, format!("Error: {error}"));
            }
        });
    }
}
