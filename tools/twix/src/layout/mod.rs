use std::{collections::HashMap, sync::Arc};

use eframe::egui::{Context, Id, Ui};
use egui_tiles::{TileId, Tree};

use crate::{SelectablePanel, backend::RobotBackend};

use self::behavior::LayoutBehavior;

mod behavior;
mod focus;
mod pane;
mod persistence;
mod preset_ui;
mod tab_bar;
mod tree;

pub use focus::FocusDirection;

fn tree_id() -> Id {
    Id::new(uuid::Uuid::new_v4())
}

pub struct TwixLayout {
    tree: Tree<SelectablePanel>,
    names: HashMap<TileId, String>,
    preset_ui: preset_ui::PresetUi,
    recovery: Option<String>,
    focused: Option<TileId>,
    selector_to_open: Option<TileId>,
    tab_to_reveal: Option<TileId>,
    focus_dirty: bool,
}

impl TwixLayout {
    pub fn dialog_open(&self) -> bool {
        self.preset_ui.dialog_open()
    }

    pub fn dialogs(&mut self, context: &Context) {
        self.preset_ui.dialogs(context);
    }

    pub fn update(&mut self, context: &Context, backend: &Arc<RobotBackend>) {
        for (_, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(panel) = tile {
                panel.update(pane::panel_ui_context(backend, context));
            }
        }
    }

    pub fn activate(&mut self, context: &Context) {
        eframe::egui::Popup::close_all(context);
        self.repair_focus(context);
        if let Some(focused) = self.focused {
            focus::request_pane_focus(context, self.tree.id(), focused);
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, backend: &Arc<RobotBackend>) {
        let mut behavior = LayoutBehavior {
            tree_id: self.tree.id(),
            root: self.tree.root,
            names: &mut self.names,
            preset_ui: &mut self.preset_ui,
            backend,
            egui_context: ui.ctx().clone(),
            focused: &mut self.focused,
            tab_to_reveal: &mut self.tab_to_reveal,
            selector_to_open: &mut self.selector_to_open,
            focus_dirty: &mut self.focus_dirty,
            dropped: false,
            requests: Vec::new(),
        };
        self.tree.ui(&mut behavior, ui);
        let dragging_tile = self.tree.dragged_id(ui.ctx()).is_some();
        if behavior.dropped {
            // A drop can empty a container after Tree::ui's initial cleanup.
            self.tree.simplify(&tree::simplification_options());
            self.tree.gc(&mut behavior);
        }
        let requests = behavior.requests;

        for request in requests {
            self.apply_request(request, backend, ui.ctx());
            ui.ctx().request_repaint();
        }
        self.names
            .retain(|id, _| self.tree.tiles.get(*id).is_some());
        if self.tree.root.is_none() {
            self.reset(backend, ui.ctx());
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
