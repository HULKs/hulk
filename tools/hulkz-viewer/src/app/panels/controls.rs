use eframe::egui;
use hulk_widgets::CompletionEdit;

use crate::app::panel_catalog::OPENABLE_PANEL_KINDS;
use crate::model::WorkerCommand;

use super::{Panel, ViewerApp};

pub(super) struct ControlsPanel;

impl Panel for ControlsPanel {
    type State = ();

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, _state: &mut Self::State) {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut app.ui.ingest_enabled, "Ingest").changed() {
                app.send_command(WorkerCommand::SetIngestEnabled(app.ui.ingest_enabled));
            }

            ui.separator();
            ui.checkbox(&mut app.shell.show_discovery, "Show Discovery");
            ui.checkbox(&mut app.shell.show_timeline, "Show Timeline");

            ui.menu_button("Open Panel", |ui| {
                for kind in OPENABLE_PANEL_KINDS {
                    if ui.button(kind.label()).clicked() {
                        app.open_panel_kind(*kind);
                        ui.close();
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Default namespace");
            let namespace_candidates = app.namespace_candidates();
            let namespace_edit = ui.add(
                CompletionEdit::new(
                    ui.id().with("default_namespace"),
                    namespace_candidates.as_slice(),
                    &mut app.ui.default_namespace_input,
                )
                .open_on_focus(true),
            );
            let submit_with_enter =
                namespace_edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if submit_with_enter || namespace_edit.lost_focus() {
                app.set_default_namespace();
            }
        });
    }
}
