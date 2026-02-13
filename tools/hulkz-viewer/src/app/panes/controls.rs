use hulk_widgets::CompletionEdit;

use crate::app::panel_prelude::{egui, Panel, PanelContext, ShellPaneKind, UiIntent};
use crate::app::workspace_panel_kind::OPENABLE_WORKSPACE_PANEL_KINDS;

pub struct ControlsPane;

impl Panel for ControlsPane {
    type State = ();

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, _state: &mut Self::State) {
        let mut ingest_enabled = ctx.app().ui.ingest_enabled;
        let mut show_discovery = ctx.app().shell.show_discovery;
        let mut show_timeline = ctx.app().shell.show_timeline;
        ui.horizontal(|ui| {
            if ui.checkbox(&mut ingest_enabled, "Ingest").changed() {
                ctx.emit(UiIntent::SetIngestEnabled(ingest_enabled));
            }

            ui.separator();
            if ui.checkbox(&mut show_discovery, "Show Discovery").changed() {
                ctx.emit(UiIntent::SetShellPaneVisible {
                    pane: ShellPaneKind::Discovery,
                    visible: show_discovery,
                });
            }
            if ui.checkbox(&mut show_timeline, "Show Timeline").changed() {
                ctx.emit(UiIntent::SetShellPaneVisible {
                    pane: ShellPaneKind::Timeline,
                    visible: show_timeline,
                });
            }

            ui.menu_button("Open Panel", |ui| {
                for kind in OPENABLE_WORKSPACE_PANEL_KINDS {
                    if ui.button(kind.label()).clicked() {
                        ctx.emit(UiIntent::OpenWorkspacePanel(*kind));
                        ui.close();
                    }
                }
            });
        });

        let namespace_candidates = ctx.app().namespace_candidates();
        let mut default_namespace_input = ctx.app().ui.default_namespace_input.clone();
        ui.horizontal(|ui| {
            ui.label("Default namespace");
            let namespace_edit = ui.add(
                CompletionEdit::new(
                    ui.id().with("default_namespace"),
                    namespace_candidates.as_slice(),
                    &mut default_namespace_input,
                )
                .open_on_focus(true),
            );
            if namespace_edit.changed() {
                ctx.emit(UiIntent::SetDefaultNamespaceDraft(
                    default_namespace_input.clone(),
                ));
            }
            let submit_with_enter =
                namespace_edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if submit_with_enter || namespace_edit.lost_focus() {
                ctx.emit(UiIntent::SetDefaultNamespaceCommitted(
                    default_namespace_input,
                ));
            }
        });
    }
}
