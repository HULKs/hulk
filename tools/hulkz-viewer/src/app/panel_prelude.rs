pub use eframe::egui;

pub use super::panel_api::{PanelContext, ShellPaneKind, UiIntent};

pub trait Panel {
    type State;

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, state: &mut Self::State);
}
