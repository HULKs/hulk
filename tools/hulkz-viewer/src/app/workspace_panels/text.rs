use hulk_widgets::CompletionEdit;
use serde::{Deserialize, Serialize};

use hulkz_stream::PlaneKind;

use crate::{
    app::{
        format_timestamp,
        panel_prelude::{egui, Panel, PanelContext},
    },
    protocol::{SourceBindingRequest, StreamId},
};

use super::shared::NamespaceSelection;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextWorkspacePanelState {
    pub(crate) id: StreamId,
    #[serde(default)]
    pub(crate) namespace_selection: NamespaceSelection,
    pub(crate) source_expression: String,
}

impl TextWorkspacePanelState {
    pub fn new(id: StreamId, source_expression: String) -> Self {
        Self {
            id,
            namespace_selection: NamespaceSelection::FollowDefault,
            source_expression,
        }
    }

    pub fn follows_default_namespace(&self) -> bool {
        matches!(self.namespace_selection, NamespaceSelection::FollowDefault)
    }

    pub fn set_namespace_override_enabled(&mut self, enabled: bool, default_namespace: &str) {
        if enabled {
            let default_namespace = default_namespace.trim().to_string();
            self.namespace_selection = NamespaceSelection::Override(default_namespace);
        } else {
            self.namespace_selection = NamespaceSelection::FollowDefault;
        }
    }

    pub fn namespace_override_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.namespace_selection {
            NamespaceSelection::FollowDefault => None,
            NamespaceSelection::Override(value) => Some(value),
        }
    }

    pub fn effective_namespace(&self, default_namespace: &str) -> Option<String> {
        let raw = match &self.namespace_selection {
            NamespaceSelection::FollowDefault => default_namespace,
            NamespaceSelection::Override(value) => value,
        };
        let namespace = raw.trim();
        if namespace.is_empty() {
            None
        } else {
            Some(namespace.to_string())
        }
    }

    pub fn binding_request(&self, default_namespace: &str) -> Option<SourceBindingRequest> {
        let namespace = self.effective_namespace(default_namespace)?;
        let path_expression = self.source_expression.trim().to_string();
        if path_expression.is_empty() {
            return None;
        }
        Some(SourceBindingRequest {
            namespace,
            plane: PlaneKind::View,
            path_expression,
        })
    }
}

pub struct TextWorkspacePane;

impl Panel for TextWorkspacePane {
    type State = TextWorkspacePanelState;

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, stream: &mut Self::State) {
        let app = ctx.app();
        ui.horizontal(|ui| {
            let mut override_enabled = !stream.follows_default_namespace();
            if ui
                .checkbox(&mut override_enabled, "Override namespace")
                .changed()
            {
                stream.set_namespace_override_enabled(override_enabled, &app.ui.default_namespace);
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

        let has_binding = stream.binding_request(&app.ui.default_namespace).is_some();
        let state = app.workspace.stream_states.get(&stream.id);
        ui.add_space(6.0);
        if let Some(state) = state {
            ui.label(egui::RichText::new(state.source_label.as_str()).weak());
            if let Some(source_stats) = &state.source_stats {
                ui.horizontal_wrapped(|ui| {
                    ui.small(format!("durable {}", source_stats.durable_len));
                    if let Some(oldest) = source_stats.durable_oldest {
                        ui.separator();
                        ui.small(format!(
                            "oldest {}",
                            format_timestamp(oldest.get_time().as_nanos())
                        ));
                    }
                    if let Some(latest) = source_stats.durable_latest {
                        ui.separator();
                        ui.small(format!(
                            "latest {}",
                            format_timestamp(latest.get_time().as_nanos())
                        ));
                    }
                    if let Some(frontier) = source_stats.ingest_frontier {
                        ui.separator();
                        ui.small(format!(
                            "ingest {}",
                            format_timestamp(frontier.get_time().as_nanos())
                        ));
                    }
                    if let Some(frontier) = source_stats.durable_frontier {
                        ui.separator();
                        ui.small(format!(
                            "durable {}",
                            format_timestamp(frontier.get_time().as_nanos())
                        ));
                    }
                });
            }
        }
        ui.separator();

        if !has_binding {
            ui.label("Unbound. Set namespace/path to subscribe.");
            return;
        }

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
            } else if app.timeline.global_timeline.is_empty() {
                ui.label("Waiting for records.");
            } else {
                ui.label("No value at the current global anchor.");
            }
        } else {
            ui.label("Stream state unavailable.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NamespaceSelection, TextWorkspacePanelState};

    #[test]
    fn follow_default_namespace_tracks_input() {
        let panel = TextWorkspacePanelState::new(1, "odometry".to_string());
        let first = panel
            .binding_request("robot-a")
            .expect("valid binding with default namespace");
        let second = panel
            .binding_request("robot-b")
            .expect("valid binding with updated namespace");

        assert_eq!(first.namespace, "robot-a");
        assert_eq!(second.namespace, "robot-b");
    }

    #[test]
    fn override_namespace_ignores_default_changes() {
        let mut panel = TextWorkspacePanelState::new(2, "odometry".to_string());
        panel.namespace_selection = NamespaceSelection::Override("robot-x".to_string());

        let first = panel
            .binding_request("robot-a")
            .expect("valid binding with namespace override");
        let second = panel
            .binding_request("robot-b")
            .expect("valid binding with namespace override");

        assert_eq!(first.namespace, "robot-x");
        assert_eq!(second.namespace, "robot-x");
    }

    #[test]
    fn binding_requires_namespace_and_path() {
        let mut panel = TextWorkspacePanelState::new(3, "".to_string());
        assert!(panel.binding_request("demo").is_none());

        panel.source_expression = "odometry".to_string();
        assert!(panel.binding_request("").is_none());
    }
}
