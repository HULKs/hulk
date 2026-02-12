use eframe::egui;

use crate::app::NamespaceSelection;

use super::{Panel, ViewerApp};

pub(super) struct DiscoveryPanel;

impl Panel for DiscoveryPanel {
    type State = ();

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, _state: &mut Self::State) {
        let discovery_namespace = app.default_namespace_input.trim();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Publishers {}", app.discovered_publishers.len()));
            ui.separator();
            ui.label(format!("Parameters {}", app.discovered_parameters.len()));
            ui.separator();
            ui.label(format!("Sessions {}", app.discovered_sessions.len()));
        });
        if !app.discovered_sessions.is_empty() {
            ui.collapsing(
                format!("Sessions ({})", app.discovered_sessions.len()),
                |ui| {
                    for session in &app.discovered_sessions {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(session.id.as_str()).monospace());
                            ui.separator();
                            ui.label(
                                egui::RichText::new(format!(
                                    "host {}",
                                    session_host_label(&session.id)
                                ))
                                .weak(),
                            );
                        });
                    }
                },
            );
        }
        ui.separator();

        if discovery_namespace.is_empty() {
            ui.add_space(6.0);
            ui.label("Set a default namespace to enable discovery.");
            return;
        }

        if app.discovered_publishers.is_empty() {
            ui.label("No publishers discovered yet.");
            return;
        }

        let mut open_panel_action: Option<(String, String)> = None;
        egui::ScrollArea::vertical()
            .id_salt("discovery_publishers")
            .show(ui, |ui| {
                for publisher in &app.discovered_publishers {
                    ui.horizontal_wrapped(|ui| {
                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(publisher.path_expression.as_str())
                                    .monospace()
                                    .strong(),
                            )
                            .sense(egui::Sense::click()),
                        );
                        ui.label(egui::RichText::new(publisher.node.as_str()).weak());
                        ui.label(
                            egui::RichText::new(scope_label_for_path(&publisher.path_expression))
                                .weak(),
                        );
                        response.context_menu(|ui| {
                            if ui.button("Open Text Panel").clicked() {
                                open_panel_action = Some((
                                    publisher.namespace.clone(),
                                    publisher.path_expression.clone(),
                                ));
                                ui.close();
                            }
                        });
                    });
                    ui.separator();
                }
            });

        if let Some((namespace, path_expression)) = open_panel_action {
            let namespace = namespace.trim().to_string();
            let selection = if namespace == app.default_namespace_input.trim() {
                NamespaceSelection::FollowDefault
            } else {
                NamespaceSelection::Override(namespace)
            };
            app.open_text_panel(selection, path_expression);
        }
    }
}

fn scope_label_for_path(path_expression: &str) -> &'static str {
    if path_expression.starts_with('/') {
        "global"
    } else if path_expression.starts_with('~') {
        "private"
    } else {
        "local"
    }
}

fn session_host_label(session_id: &str) -> &str {
    session_id
        .split_once('@')
        .map(|(_, host)| host)
        .unwrap_or("<unknown>")
}
