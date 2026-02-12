use eframe::egui;

use super::{ParameterPanelTab, TextPanelTab, ViewerApp};

mod controls;
mod discovery;
mod parameters;
mod status;
mod text;
mod timeline;

pub(super) trait Panel {
    type State;

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, state: &mut Self::State);
}

pub(super) fn draw_controls(app: &mut ViewerApp, ui: &mut egui::Ui) {
    let mut unit = ();
    controls::ControlsPanel::draw(app, ui, &mut unit);
}

pub(super) fn draw_timeline_panel(app: &mut ViewerApp, ui: &mut egui::Ui) {
    let mut unit = ();
    timeline::TimelinePanel::draw(app, ui, &mut unit);
}

pub(super) fn draw_discovery_panel(app: &mut ViewerApp, ui: &mut egui::Ui) {
    let mut unit = ();
    discovery::DiscoveryPanel::draw(app, ui, &mut unit);
}

pub(super) fn draw_parameters_panel(
    app: &mut ViewerApp,
    ui: &mut egui::Ui,
    panel: &mut ParameterPanelTab,
) {
    parameters::ParametersPanel::draw(app, ui, panel);
}

pub(super) fn draw_text_panel(app: &mut ViewerApp, ui: &mut egui::Ui, stream: &mut TextPanelTab) {
    text::TextPanel::draw(app, ui, stream);
}

pub(super) fn draw_status_bar(app: &mut ViewerApp, ui: &mut egui::Ui) {
    let mut unit = ();
    status::StatusPanel::draw(app, ui, &mut unit);
}
