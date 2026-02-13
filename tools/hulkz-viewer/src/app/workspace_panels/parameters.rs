use hulk_widgets::CompletionEdit;
use serde::{Deserialize, Serialize};

use crate::app::panel_prelude::{egui, Panel, PanelContext, UiIntent};
use crate::protocol::ParameterReference;

use super::shared::NamespaceSelection;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametersWorkspacePanelState {
    pub(crate) id: u64,
    #[serde(default)]
    pub(crate) namespace_selection: NamespaceSelection,
    pub(crate) node_input: String,
    pub(crate) path_input: String,
    pub(crate) editor_text: String,
    #[serde(skip)]
    pub(crate) selected_parameter_reference: Option<ParameterReference>,
    #[serde(default)]
    pub(crate) status: Option<ParametersPanelStatus>,
}

impl ParametersWorkspacePanelState {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            namespace_selection: NamespaceSelection::FollowDefault,
            node_input: String::new(),
            path_input: String::new(),
            editor_text: String::new(),
            selected_parameter_reference: None,
            status: None,
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

    pub fn set_selected_parameter_reference(&mut self, target: ParameterReference) {
        self.node_input = target.node.clone();
        self.path_input = target.path_expression.clone();
        self.selected_parameter_reference = Some(target);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametersPanelStatus {
    pub(crate) success: bool,
    pub(crate) message: String,
}

pub struct ParametersWorkspacePane;

impl Panel for ParametersWorkspacePane {
    type State = ParametersWorkspacePanelState;

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, panel: &mut Self::State) {
        let default_namespace = ctx.app().ui.default_namespace.clone();
        ui.horizontal(|ui| {
            let mut override_enabled =
                !matches!(panel.namespace_selection, NamespaceSelection::FollowDefault);
            if ui
                .checkbox(&mut override_enabled, "Override namespace")
                .changed()
            {
                panel.namespace_selection = if override_enabled {
                    NamespaceSelection::Override(default_namespace.trim().to_string())
                } else {
                    NamespaceSelection::FollowDefault
                };
                panel.status = None;
            }

            if let NamespaceSelection::Override(namespace) = &mut panel.namespace_selection {
                ui.text_edit_singleline(namespace);
            }
        });

        let Some(namespace) = panel.effective_namespace(&default_namespace) else {
            ui.add_space(6.0);
            ui.label("Set a default namespace first.");
            return;
        };
        let namespace_filter = namespace.as_str();

        let mut trigger_auto_load = false;
        ui.horizontal(|ui| {
            ui.label("Node");
            let node_candidates = ctx.app().parameter_node_candidates(panel);
            let node_edit = ui.add(
                CompletionEdit::new(
                    ui.id().with(("parameter_node", panel.id)),
                    node_candidates.as_slice(),
                    &mut panel.node_input,
                )
                .open_on_focus(true),
            );
            if node_edit.changed() {
                panel.status = None;
                trigger_auto_load = true;
            }
            let node_submit =
                node_edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if node_submit || node_edit.lost_focus() {
                panel.node_input = panel.node_input.trim().to_string();
                trigger_auto_load = true;
            }

            ui.label("Path")
                .on_hover_text("DSL: gain | /global/param | ~node/private_param");
            let path_candidates = ctx.app().parameter_path_candidates(panel);
            let path_edit = ui.add(
                CompletionEdit::new(
                    ui.id().with(("parameter_path", panel.id)),
                    path_candidates.as_slice(),
                    &mut panel.path_input,
                )
                .open_on_focus(true),
            );
            if path_edit.changed() {
                panel.status = None;
                trigger_auto_load = true;
            }
            let path_submit =
                path_edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if path_submit || path_edit.lost_focus() {
                panel.path_input = panel.path_input.trim().to_string();
                trigger_auto_load = true;
            }
        });

        if trigger_auto_load {
            let target_result = ctx.app().parameter_reference_from_inputs(panel);
            if let Ok(target) = target_result {
                let should_read = panel.selected_parameter_reference.as_ref() != Some(&target);
                panel.set_selected_parameter_reference(target.clone());
                if should_read {
                    panel.status = None;
                    ctx.emit(UiIntent::ReadParameter {
                        panel_id: panel.id,
                        target,
                    });
                }
            } else if panel.node_input.trim().is_empty() && panel.path_input.trim().is_empty() {
                panel.selected_parameter_reference = None;
            }
        }

        let discovered_parameter_count = ctx
            .app()
            .discovery
            .parameters
            .iter()
            .filter(|parameter| parameter.namespace == namespace_filter)
            .count();
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                let apply_target = ctx.app().parameter_reference_from_inputs(panel);
                match apply_target {
                    Ok(target) => {
                        panel.set_selected_parameter_reference(target.clone());
                        ctx.emit(UiIntent::ApplyParameter {
                            panel_id: panel.id,
                            target,
                            value_json: panel.editor_text.clone(),
                        });
                    }
                    Err(message) => {
                        panel.status = Some(ParametersPanelStatus {
                            success: false,
                            message,
                        });
                    }
                }
            }
            ui.separator();
            ui.small(format!(
                "{discovered_parameter_count} parameter(s) discovered"
            ));
        });

        ui.separator();
        if panel.selected_parameter_reference.is_none() {
            ui.label(
                egui::RichText::new("Unbound. Enter node/path to load a parameter value.").weak(),
            );
            ui.add_space(4.0);
        }
        ui.add(
            egui::TextEdit::multiline(&mut panel.editor_text)
                .font(egui::TextStyle::Monospace)
                .desired_rows(14),
        );
        if let Some(status) = &panel.status {
            if status.success {
                ui.colored_label(egui::Color32::GREEN, &status.message);
            } else {
                ui.colored_label(egui::Color32::RED, &status.message);
            }
        }
    }
}
