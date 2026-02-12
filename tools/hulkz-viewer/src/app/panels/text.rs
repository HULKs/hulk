use eframe::egui;
use hulk_widgets::CompletionEdit;

use crate::app::{format_timestamp, TextPanelTab};

use super::{Panel, ViewerApp};

pub(super) struct TextPanel;

impl Panel for TextPanel {
    type State = TextPanelTab;

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, stream: &mut Self::State) {
        ui.horizontal(|ui| {
            let mut override_enabled = !stream.follows_default_namespace();
            if ui
                .checkbox(&mut override_enabled, "Override namespace")
                .changed()
            {
                stream
                    .set_namespace_override_enabled(override_enabled, &app.default_namespace_input);
            }

            if let Some(override_namespace) = stream.namespace_override_text_mut() {
                ui.text_edit_singleline(override_namespace);
            }

            ui.label("Path")
                .on_hover_text("DSL: odometry | /fleet/topic | ~node/private_topic");
            let candidates = app.source_path_candidates(stream);
            ui.add(
                CompletionEdit::new(
                    ui.id().with(("text_path", stream.id)),
                    candidates.as_slice(),
                    &mut stream.source_expression,
                )
                .open_on_focus(true),
            );
        });

        let state = app.stream_states.get(&stream.id);
        ui.add_space(6.0);
        if let Some(state) = state {
            ui.label(egui::RichText::new(state.source_label.as_str()).weak());
        }
        ui.separator();

        if let Some(state) = state {
            if let Some(record) = &state.current_record {
                ui.label(egui::RichText::new(format_timestamp(record.timestamp_nanos)).monospace());
                ui.separator();

                let mut body = record
                    .json_pretty
                    .clone()
                    .or_else(|| record.raw_fallback.clone())
                    .unwrap_or_else(|| "<empty payload>".to_string());

                ui.add(
                    egui::TextEdit::multiline(&mut body)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(24)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(body);
                }
            } else if app.global_timeline.is_empty() {
                ui.label("Waiting for records.");
            } else {
                ui.label("No value at current anchor.");
            }
        } else {
            ui.label("Stream state unavailable.");
        }
    }
}
