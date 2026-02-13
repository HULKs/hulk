use crate::app::{
    panel_prelude::{egui, Panel, PanelContext, UiIntent},
    workspace_panels::NamespaceSelection,
};

pub struct DiscoveryPane;

impl Panel for DiscoveryPane {
    type State = ();

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, _state: &mut Self::State) {
        let discovery_namespace = ctx.app().ui.default_namespace.trim().to_string();
        let publisher_count = ctx.app().discovery.publishers.len();
        let parameter_count = ctx.app().discovery.parameters.len();
        let session_count = ctx.app().discovery.sessions.len();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Publishers {publisher_count}"));
            ui.separator();
            ui.label(format!("Parameters {parameter_count}"));
            ui.separator();
            ui.label(format!("Sessions {session_count}"));
        });
        let sessions = ctx.app().discovery.sessions.clone();
        if !sessions.is_empty() {
            ui.collapsing(format!("Sessions ({})", sessions.len()), |ui| {
                for session in &sessions {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(session.id.as_str()).monospace());
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "host {}",
                                    session_host_label(&session.id)
                                ))
                                .weak(),
                            );
                        });
                    });
                }
            });
        }
        ui.separator();

        if discovery_namespace.is_empty() {
            ui.add_space(6.0);
            ui.label("Set a default namespace to discover publishers and parameters.");
            ui.label(egui::RichText::new("Session discovery remains active globally.").weak());
            return;
        }

        let publishers = ctx.app().discovery.publishers.clone();
        if publishers.is_empty() {
            ui.label("No publishers discovered yet.");
            return;
        }

        let mut open_panel_action: Option<(String, String)> = None;
        egui::ScrollArea::vertical()
            .id_salt("discovery_publishers")
            .show(ui, |ui| {
                for publisher in &publishers {
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
            let default_namespace = ctx.app().ui.default_namespace.clone();
            let selection = if namespace == default_namespace.trim() {
                NamespaceSelection::FollowDefault
            } else {
                NamespaceSelection::Override(namespace)
            };
            ctx.emit(UiIntent::OpenTextWorkspacePanel {
                namespace_selection: selection,
                path_expression,
            });
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
