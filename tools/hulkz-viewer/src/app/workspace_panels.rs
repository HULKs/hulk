use super::{
    panel_api::{PanelContext, UiIntent},
    panel_prelude::Panel,
    state::ViewerApp,
};
use eframe::egui;

pub(crate) use parameters::{ParametersPanelStatus, ParametersWorkspacePanelState};
pub(crate) use shared::NamespaceSelection;
pub(crate) use text::TextWorkspacePanelState;

mod parameters;
mod shared;
mod text;

pub fn draw_workspace_parameters_panel(
    app: &mut ViewerApp,
    intents: &mut Vec<UiIntent>,
    ui: &mut egui::Ui,
    panel: &mut ParametersWorkspacePanelState,
) {
    let mut ctx = PanelContext::new(app, intents);
    parameters::ParametersWorkspacePane::draw(&mut ctx, ui, panel);
}

pub fn draw_workspace_text_panel(
    app: &mut ViewerApp,
    intents: &mut Vec<UiIntent>,
    ui: &mut egui::Ui,
    stream: &mut TextWorkspacePanelState,
) {
    let mut ctx = PanelContext::new(app, intents);
    text::TextWorkspacePane::draw(&mut ctx, ui, stream);
}
