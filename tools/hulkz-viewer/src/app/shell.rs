use eframe::egui;

use super::{panel_api::UiIntent, panes, state::ViewerApp};

impl ViewerApp {
    pub fn render_shell_panes(&mut self, ctx: &egui::Context, ui_intents: &mut Vec<UiIntent>) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            panes::draw_shell_controls_pane(self, ui_intents, ui);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            panes::draw_shell_status_pane(self, ui_intents, ui);
        });

        if self.shell.show_timeline {
            egui::TopBottomPanel::bottom("timeline_shell")
                .resizable(true)
                .show(ctx, |ui| {
                    panes::draw_shell_timeline_pane(self, ui_intents, ui);
                });
        }

        if self.shell.show_discovery {
            egui::SidePanel::left("discovery_shell")
                .resizable(true)
                .show(ctx, |ui| {
                    panes::draw_shell_discovery_pane(self, ui_intents, ui);
                });
        }
    }
}
