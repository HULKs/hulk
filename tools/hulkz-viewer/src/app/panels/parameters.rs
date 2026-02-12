use eframe::egui::{self, Color32};
use hulk_widgets::CompletionEdit;

use crate::app::{NamespaceSelection, ParameterPanelStatus, ParameterPanelTab};
use crate::model::WorkerCommand;

use super::{Panel, ViewerApp};

pub(super) struct ParametersPanel;

impl Panel for ParametersPanel {
    type State = ParameterPanelTab;

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, panel: &mut Self::State) {
        ui.horizontal(|ui| {
            let mut override_enabled =
                !matches!(panel.namespace_selection, NamespaceSelection::FollowDefault);
            if ui
                .checkbox(&mut override_enabled, "Override namespace")
                .changed()
            {
                panel.namespace_selection = if override_enabled {
                    NamespaceSelection::Override(app.default_namespace_input.trim().to_string())
                } else {
                    NamespaceSelection::FollowDefault
                };
                panel.status = None;
            }

            if let NamespaceSelection::Override(namespace) = &mut panel.namespace_selection {
                ui.text_edit_singleline(namespace);
            }
        });

        let Some(namespace) = panel.effective_namespace(&app.default_namespace_input) else {
            ui.add_space(6.0);
            ui.label("Set a default namespace first.");
            return;
        };
        let namespace_filter = namespace.as_str();

        let mut trigger_auto_load = false;
        ui.horizontal(|ui| {
            ui.label("Node");
            let node_candidates = app.parameter_node_candidates(panel);
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
            let path_candidates = app.parameter_path_candidates(panel);
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
            if let Ok(target) = app.parameter_reference_from_inputs(panel) {
                let should_read = panel.selected_parameter_reference.as_ref() != Some(&target);
                panel.set_selected_parameter_reference(target.clone());
                if should_read {
                    panel.status = None;
                    app.send_command(WorkerCommand::ReadParameter(target));
                }
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                match app.parameter_reference_from_inputs(panel) {
                    Ok(target) => {
                        panel.set_selected_parameter_reference(target.clone());
                        app.send_command(WorkerCommand::SetParameter {
                            target,
                            value_json: panel.editor_text.clone(),
                        });
                    }
                    Err(message) => {
                        panel.status = Some(ParameterPanelStatus {
                            success: false,
                            message,
                        });
                    }
                }
            }
            ui.separator();
            ui.small(format!(
                "{} parameter(s) discovered",
                app.discovered_parameters
                    .iter()
                    .filter(|parameter| parameter.namespace == namespace_filter)
                    .count()
            ));
        });

        ui.separator();
        ui.add(
            egui::TextEdit::multiline(&mut panel.editor_text)
                .font(egui::TextStyle::Monospace)
                .desired_rows(14),
        );
        if let Some(status) = &panel.status {
            if status.success {
                ui.colored_label(Color32::GREEN, &status.message);
            } else {
                ui.colored_label(Color32::RED, &status.message);
            }
        }
    }
}
