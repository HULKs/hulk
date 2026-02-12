use eframe::egui;
use hulk_widgets::CompletionEdit;

use crate::model::WorkerCommand;

use super::{Panel, ViewerApp};
use crate::app::panel_catalog::OPENABLE_PANEL_KINDS;

pub(super) struct ControlsPanel;

impl Panel for ControlsPanel {
    type State = ();

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, _state: &mut Self::State) {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut app.ingest_enabled, "Ingest").changed() {
                app.send_command(WorkerCommand::SetIngestEnabled(app.ingest_enabled));
            }

            ui.menu_button("Open Panel", |ui| {
                for kind in OPENABLE_PANEL_KINDS {
                    let should_enable = !kind.is_singleton() || !app.has_panel_kind(*kind);
                    if ui
                        .add_enabled(should_enable, egui::Button::new(kind.label()))
                        .clicked()
                    {
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
                    &mut app.default_namespace_input,
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
