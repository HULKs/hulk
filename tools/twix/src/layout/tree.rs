use std::sync::Arc;

use eframe::egui::Context;
use egui_tiles::{Container, Linear, LinearDir, SimplificationOptions, Tile, TileId, Tiles, Tree};
use log::error;

use crate::{PanelKind, SelectablePanel, backend::RobotBackend};

use super::{TwixLayout, focus::request_pane_focus, pane::panel_creation_context, tree_id};

pub(super) enum LayoutRequest {
    Add {
        tabs: TileId,
        panel: PanelKind,
    },
    Replace {
        pane: TileId,
        panel: PanelKind,
    },
    Close(TileId),
    Import {
        tabs: TileId,
        title: String,
        saved: String,
    },
    Blank(TileId),
    Save(TileId),
}

impl TwixLayout {
    pub fn open_selector(&mut self, egui_context: &Context) {
        self.repair_focus(egui_context);
        self.selector_to_open = self.focused;
    }

    pub fn open_tab(&mut self, backend: &Arc<RobotBackend>, egui_context: &Context) {
        let Some(tabs_id) = self.focused_tabs() else {
            return;
        };
        self.insert_tab(
            tabs_id,
            SelectablePanel::text(backend, egui_context),
            egui_context,
        );
    }

    pub fn open_split(&mut self, backend: &Arc<RobotBackend>, egui_context: &Context) {
        let Some(tabs_id) = self.focused_tabs() else {
            return;
        };
        let direction = self
            .tree
            .tiles
            .rect(tabs_id)
            .map_or(LinearDir::Horizontal, |rect| {
                if rect.height() > rect.width() {
                    LinearDir::Vertical
                } else {
                    LinearDir::Horizontal
                }
            });
        let Some(old_tabs) = self.tree.tiles.remove(tabs_id) else {
            return;
        };

        let old_tabs = self.tree.tiles.insert_new(old_tabs);
        let new_pane = self
            .tree
            .tiles
            .insert_pane(SelectablePanel::text(backend, egui_context));
        let new_tabs = self.tree.tiles.insert_tab_tile(vec![new_pane]);
        self.tree.tiles.insert(
            tabs_id,
            Tile::Container(Container::Linear(Linear::new_binary(
                direction,
                [old_tabs, new_tabs],
                0.5,
            ))),
        );
        self.set_focus(new_pane, egui_context);
    }

    pub fn duplicate_focused(&mut self, backend: &Arc<RobotBackend>, egui_context: &Context) {
        let Some(focused) = self.focused else {
            return;
        };
        let Some(tabs_id) = self.focused_tabs() else {
            return;
        };
        let Some(Tile::Pane(tab)) = self.tree.tiles.get(focused) else {
            return;
        };
        let saved = tab.save();
        let pane = match SelectablePanel::restore(backend, &saved, egui_context) {
            Ok(pane) => pane,
            Err(error) => {
                error!("failed to duplicate tab: {error:#}");
                return;
            }
        };
        self.insert_tab(tabs_id, pane, egui_context);
    }

    pub fn close_focused(&mut self, backend: &Arc<RobotBackend>, egui_context: &Context) {
        if let Some(focused) = self.focused {
            self.close_tile(focused, backend, egui_context);
        }
    }

    pub fn reset(&mut self, backend: &Arc<RobotBackend>, egui_context: &Context) {
        let recovery = self.recovery.take();
        *self = Self::new(egui_context, backend);
        self.recovery = recovery;
        self.activate(egui_context);
    }

    pub fn new(egui_context: &Context, backend: &Arc<RobotBackend>) -> Self {
        let tree = Tree::new_tabs(
            tree_id(),
            vec![SelectablePanel::text(backend, egui_context)],
        );
        let focused = first_pane(&tree);
        Self {
            tree,
            names: Default::default(),
            preset_ui: Default::default(),
            recovery: None,
            focused,
            selector_to_open: None,
            tab_to_reveal: None,
            focus_dirty: false,
        }
    }

    pub(super) fn apply_request(
        &mut self,
        request: LayoutRequest,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) {
        match request {
            LayoutRequest::Add { tabs, panel } => {
                if !matches!(
                    self.tree.tiles.get(tabs),
                    Some(Tile::Container(Container::Tabs(_)))
                ) {
                    return;
                }
                let panel = panel.create(panel_creation_context(backend, None, egui_context));
                self.insert_tab(tabs, panel, egui_context);
            }
            LayoutRequest::Replace { pane, panel } => {
                let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(pane) else {
                    return;
                };
                *pane = panel.create(panel_creation_context(backend, None, egui_context));
            }
            LayoutRequest::Close(tile_id) => {
                self.close_tile(tile_id, backend, egui_context);
            }
            LayoutRequest::Import { tabs, title, saved } => {
                let result = Self::from_serialized(&saved, backend, egui_context)
                    .and_then(|layout| self.insert_layout(tabs, layout, title));
                match result {
                    Ok(_) => self.activate(egui_context),
                    Err(error) => self.preset_ui.error = Some(format!("{error:#}")),
                }
            }
            LayoutRequest::Blank(tabs) => {
                match self.insert_layout(tabs, Self::new(egui_context, backend), "Workspace".into())
                {
                    Ok(_) => self.activate(egui_context),
                    Err(error) => self.preset_ui.error = Some(format!("{error:#}")),
                }
            }
            LayoutRequest::Save(tile) => match self.snapshot_subtree(tile) {
                Ok(saved) => self.preset_ui.save(self.title(tile), saved),
                Err(error) => self.preset_ui.error = Some(format!("{error:#}")),
            },
        }
    }

    fn insert_tab(
        &mut self,
        tabs_id: TileId,
        pane: SelectablePanel,
        egui_context: &Context,
    ) -> TileId {
        let pane_id = self.tree.tiles.insert_pane(pane);
        let tabs = self
            .tree
            .tiles
            .get_mut(tabs_id)
            .and_then(|tile| match tile {
                Tile::Container(Container::Tabs(tabs)) => Some(tabs),
                _ => None,
            })
            .expect("tab group should identify a tabs container");
        tabs.add_child(pane_id);
        tabs.set_active(pane_id);
        self.focused = Some(pane_id);
        self.tab_to_reveal = Some(pane_id);
        request_pane_focus(egui_context, self.tree.id(), pane_id);
        pane_id
    }

    pub(super) fn close_tile(
        &mut self,
        tile_id: TileId,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) {
        if self.tree.tiles.get(tile_id).is_none() {
            return;
        }
        let fallback = self.tree.tiles.parent_of(tile_id).and_then(|parent| {
            match self.tree.tiles.get(parent) {
                Some(Tile::Container(Container::Tabs(tabs))) => {
                    let index = tabs.children.iter().position(|child| *child == tile_id)?;
                    tabs.children[..index]
                        .iter()
                        .rev()
                        .chain(&tabs.children[index + 1..])
                        .copied()
                        .find(|child| self.tree.tiles.is_visible(*child))
                }
                _ => None,
            }
        });
        self.tree.remove_recursively(tile_id);
        let focus_was_removed = self
            .focused
            .is_some_and(|focused| self.tree.tiles.get(focused).is_none());
        self.tree.simplify(&simplification_options());

        if first_pane(&self.tree).is_none() {
            self.reset(backend, egui_context);
        } else if focus_was_removed
            && let Some(fallback) = fallback
            && let Some(fallback) = active_pane_in_tile(&self.tree.tiles, fallback)
        {
            self.set_focus(fallback, egui_context);
        } else {
            self.repair_focus(egui_context);
        }
    }

    pub(super) fn focused_tabs(&self) -> Option<TileId> {
        let focused = self.focused?;
        let parent = self.tree.tiles.parent_of(focused)?;
        matches!(
            self.tree.tiles.get(parent),
            Some(Tile::Container(Container::Tabs(_)))
        )
        .then_some(parent)
    }

    pub(super) fn set_focus(&mut self, tile_id: TileId, egui_context: &Context) {
        self.focused = Some(tile_id);
        self.tree.make_active(|candidate, _| candidate == tile_id);
        request_pane_focus(egui_context, self.tree.id(), tile_id);
    }

    pub(super) fn repair_focus(&mut self, egui_context: &Context) {
        if let Some(focused) = self.focused
            && matches!(self.tree.tiles.get(focused), Some(Tile::Pane(_)))
            && self.tree.active_tiles().contains(&focused)
        {
            return;
        }
        self.focused = first_pane(&self.tree);
        if let Some(focused) = self.focused {
            self.tree.make_active(|tile_id, _| tile_id == focused);
            request_pane_focus(egui_context, self.tree.id(), focused);
        }
    }
}

pub(super) fn simplification_options() -> SimplificationOptions {
    SimplificationOptions {
        all_panes_must_have_tabs: true,
        prune_single_child_tabs: false,
        prune_single_child_containers: false,
        join_nested_linear_containers: false,
        ..Default::default()
    }
}

pub(super) fn first_pane(tree: &Tree<SelectablePanel>) -> Option<TileId> {
    tree.active_tiles()
        .into_iter()
        .find(|tile_id| matches!(tree.tiles.get(*tile_id), Some(Tile::Pane(_))))
}

pub(super) fn active_pane_in_tile(
    tiles: &Tiles<SelectablePanel>,
    tile_id: TileId,
) -> Option<TileId> {
    match tiles.get(tile_id)? {
        Tile::Pane(_) => Some(tile_id),
        Tile::Container(container) => container
            .active_children(tiles)
            .find_map(|child| active_pane_in_tile(tiles, child)),
    }
}
