use super::{panel_api::PanelContext, panel_prelude::Panel, state::ViewerApp};
use eframe::egui;

mod controls;
mod discovery;
mod status;
mod timeline;

pub fn draw_shell_controls_pane(
    app: &mut ViewerApp,
    intents: &mut Vec<super::panel_api::UiIntent>,
    ui: &mut egui::Ui,
) {
    let mut unit = ();
    let mut ctx = PanelContext::new(app, intents);
    controls::ControlsPane::draw(&mut ctx, ui, &mut unit);
}

pub fn draw_shell_timeline_pane(
    app: &mut ViewerApp,
    intents: &mut Vec<super::panel_api::UiIntent>,
    ui: &mut egui::Ui,
) {
    let mut unit = ();
    let mut ctx = PanelContext::new(app, intents);
    timeline::TimelinePane::draw(&mut ctx, ui, &mut unit);
}

pub fn draw_shell_discovery_pane(
    app: &mut ViewerApp,
    intents: &mut Vec<super::panel_api::UiIntent>,
    ui: &mut egui::Ui,
) {
    let mut unit = ();
    let mut ctx = PanelContext::new(app, intents);
    discovery::DiscoveryPane::draw(&mut ctx, ui, &mut unit);
}

pub fn draw_shell_status_pane(
    app: &mut ViewerApp,
    intents: &mut Vec<super::panel_api::UiIntent>,
    ui: &mut egui::Ui,
) {
    let mut unit = ();
    let mut ctx = PanelContext::new(app, intents);
    status::StatusPane::draw(&mut ctx, ui, &mut unit);
}
