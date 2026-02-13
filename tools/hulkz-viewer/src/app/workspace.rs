use std::panic::{catch_unwind, AssertUnwindSafe};

use eframe::egui;
use egui_dock::{DockArea, DockState};
use tracing::warn;

use super::{
    layout::{initial_dock_state, sanitize_dock_splits, WorkspacePanelHost},
    panel_api::UiIntent,
    state::ViewerApp,
    workspace_panel::WorkspacePanel,
    workspace_panels::{ParametersWorkspacePanelState, TextWorkspacePanelState},
};

impl ViewerApp {
    pub fn render_workspace(&mut self, ctx: &egui::Context, ui_intents: &mut Vec<UiIntent>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut dock_state = std::mem::replace(
                &mut self.workspace.dock_state,
                DockState::new(vec![WorkspacePanel::Text(TextWorkspacePanelState::new(
                    0,
                    self.config.source_expression.clone(),
                ))]),
            );
            let text_panel_count = dock_state
                .iter_all_tabs()
                .filter(|(_, tab)| matches!(tab, WorkspacePanel::Text(_)))
                .count();
            if sanitize_dock_splits(&mut dock_state) {
                warn!("sanitized dock split fractions before render");
            }
            let dock_render = catch_unwind(AssertUnwindSafe(|| {
                let mut tab_viewer = WorkspacePanelHost {
                    app: self,
                    ui_intents,
                    text_panel_count,
                };
                DockArea::new(&mut dock_state)
                    .show_close_buttons(true)
                    .secondary_button_on_modifier(false)
                    .show_inside(ui, &mut tab_viewer);
            }));
            match dock_render {
                Ok(()) => {
                    self.workspace.dock_state = dock_state;
                }
                Err(_) => {
                    warn!("dock rendering panicked; resetting dock layout");
                    self.workspace.dock_state = initial_dock_state(
                        TextWorkspacePanelState::new(0, self.config.source_expression.clone()),
                        ParametersWorkspacePanelState::new(0),
                    );
                    self.ui.last_error =
                        Some("Dock layout was invalid and has been reset.".to_string());
                }
            }
        });
    }
}
