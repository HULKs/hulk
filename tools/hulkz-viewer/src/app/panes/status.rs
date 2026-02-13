use crate::app::{
    format_timestamp,
    panel_prelude::{egui, Panel, PanelContext},
};

pub struct StatusPane;

impl Panel for StatusPane {
    type State = ();

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, _state: &mut Self::State) {
        let app = ctx.app();
        let total_records = app.timeline.global_timeline.len();
        let queued_events = app.runtime.event_rx.len();
        let pending_commands = app.runtime.pending_commands.len();
        let avg_event_bytes = if app.ui.frame_processed_events > 0 {
            app.ui.frame_processed_event_bytes / app.ui.frame_processed_events
        } else {
            0
        };
        let queued_event_bytes = queued_events.saturating_mul(avg_event_bytes);
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Ready: {}", app.ui.ready));
            ui.separator();
            ui.label(format!("Streams: {}", app.workspace.stream_states.len()));
            ui.separator();
            ui.label(format!("Timeline points: {total_records}"));
            ui.separator();
            ui.label(format!("Follow live: {}", app.ui.follow_live));
            ui.separator();
            ui.label(format!("Frame {:.2} ms", app.ui.frame_last_ms));
            ui.separator();
            ui.label(format!("Frame EMA {:.2} ms", app.ui.frame_ema_ms));
            ui.separator();
            ui.label(format!(
                "Events/frame {} ({:.1} KiB)",
                app.ui.frame_processed_events,
                app.ui.frame_processed_event_bytes as f32 / 1024.0
            ));
            ui.separator();
            ui.label(format!(
                "Queued events {} ({:.1} KiB)",
                queued_events,
                queued_event_bytes as f32 / 1024.0
            ));
            ui.separator();
            ui.label(format!("Pending commands: {pending_commands}"));
            if let Some(anchor) = app.current_anchor_nanos() {
                ui.separator();
                ui.label(format!("Anchor: {}", format_timestamp(anchor)));
            }

            if let Some(backend_stats) = &app.ui.backend_stats {
                ui.separator();
                ui.label(format!("Active sources: {}", backend_stats.active_sources));
                ui.separator();
                ui.label(format!("Queue depth: {}", backend_stats.writer_queue_depth));
            }

            if let Some(error) = &app.ui.last_error {
                ui.separator();
                ui.colored_label(egui::Color32::RED, format!("Error: {error}"));
            }
        });
    }
}
