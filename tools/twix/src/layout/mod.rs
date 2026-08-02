use std::sync::Arc;

use eframe::egui::{Popup, Ui};
use egui_tiles::{TileId, Tree};

use crate::backend::RobotBackend;

use self::{behavior::LayoutBehavior, pane::PanelPane};

mod behavior;
mod focus;
mod pane;
mod tab_bar;
mod tree;

#[cfg(test)]
mod tests;

pub use focus::FocusDirection;

const TREE_ID: &str = "twix-tile-tree";

pub struct TwixLayout {
    tree: Tree<PanelPane>,
    focused: Option<TileId>,
    selector_to_open: Option<TileId>,
    tab_to_reveal: Option<TileId>,
    focus_dirty: bool,
}

impl TwixLayout {
    pub fn ui(&mut self, ui: &mut Ui, backend: &Arc<RobotBackend>) {
        let mut behavior = LayoutBehavior {
            backend,
            egui_context: ui.ctx().clone(),
            focused: &mut self.focused,
            tab_to_reveal: &mut self.tab_to_reveal,
            selector_to_open: &mut self.selector_to_open,
            add_button_after: None,
            focus_dirty: &mut self.focus_dirty,
            requests: Vec::new(),
        };
        self.tree.ui(&mut behavior, ui);
        let requests = behavior.requests;
        let dragging_tile = self.tree.dragged_id(ui.ctx()).is_some();
        if dragging_tile {
            Popup::close_all(ui.ctx());
        }

        for request in requests {
            self.apply_request(request, backend, ui.ctx());
            ui.ctx().request_repaint();
        }
        if self.focus_dirty && !dragging_tile {
            self.focus_dirty = false;
            let previous_focus = self.focused;
            self.repair_focus(ui.ctx());
            if self.focused != previous_focus {
                ui.request_repaint();
            }
        }
        if self.selector_to_open.is_some_and(|target| {
            !matches!(self.tree.tiles.get(target), Some(egui_tiles::Tile::Pane(_)))
        }) {
            self.selector_to_open = self.focused;
            ui.request_repaint();
        }
    }
}
