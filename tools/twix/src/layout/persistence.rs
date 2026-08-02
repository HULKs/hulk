use std::sync::Arc;

use color_eyre::{
    Result,
    eyre::{WrapErr as _, bail},
};
use eframe::{CreationContext, Storage, egui::Context};
use egui_tiles::{Tile, TileId, Tiles, Tree};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend::RobotBackend;

use super::{TREE_ID, TwixLayout, focus::request_pane_focus, pane::PanelPane, tree::first_pane};

const STORAGE_KEY: &str = "tile_layout";
pub(super) const MAX_RESTORED_TILE_ID: u64 = 1 << 20;

#[derive(Serialize)]
pub(super) struct SavedLayout<'a> {
    pub(super) tree: &'a Tree<PanelPane>,
    pub(super) focused: Option<TileId>,
}

#[derive(Deserialize)]
struct LoadedLayout {
    tree: Tree<Value>,
    focused: Option<TileId>,
}

impl TwixLayout {
    pub fn load(creation_context: &CreationContext, backend: &Arc<RobotBackend>) -> Self {
        if let Some(serialized) = creation_context
            .storage
            .and_then(|storage| storage.get_string(STORAGE_KEY))
        {
            match Self::from_serialized(&serialized, backend, &creation_context.egui_ctx) {
                Ok(layout) => return layout,
                Err(error) => error!("failed to load tile layout: {error:#}"),
            }
        }

        Self::new(&creation_context.egui_ctx, backend)
    }

    pub fn save(&self, storage: &mut dyn Storage) {
        let layout = SavedLayout {
            tree: &self.tree,
            focused: self.focused,
        };
        match serde_json::to_string(&layout) {
            Ok(serialized) => storage.set_string(STORAGE_KEY, serialized),
            Err(error) => error!("failed to serialize tile layout: {error}"),
        }
    }

    pub(super) fn from_serialized(
        serialized: &str,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) -> Result<Self> {
        let loaded: LoadedLayout =
            serde_json::from_str(serialized).wrap_err("failed to deserialize tile layout")?;
        let Some(root) = loaded.tree.root else {
            bail!("tile layout has no root");
        };

        let highest_id = loaded
            .tree
            .tiles
            .tile_ids()
            .map(|tile_id| tile_id.0)
            .max()
            .unwrap_or_default();
        if MAX_RESTORED_TILE_ID < highest_id {
            bail!("persisted tile ID {highest_id} exceeds the supported limit");
        }

        let mut tiles = Tiles::default();
        // Explicit insertion preserves persisted IDs but does not advance Tiles' private allocator.
        while tiles.next_free_id().0 < highest_id {}

        for (tile_id, tile) in loaded.tree.tiles.iter() {
            let tile = match tile {
                Tile::Pane(value) => Tile::Pane(
                    PanelPane::restore(backend, value, egui_context)
                        .wrap_err_with(|| format!("failed to restore panel in tile {tile_id:?}"))?,
                ),
                Tile::Container(container) => Tile::Container(container.clone()),
            };
            tiles.insert(*tile_id, tile);
            tiles.set_visible(*tile_id, loaded.tree.tiles.is_visible(*tile_id));
        }

        let mut tree = Tree::new(TREE_ID, root, tiles);
        let Some(default_focus) = first_pane(&tree) else {
            bail!("tile layout has no active panels");
        };
        let mut focused = loaded
            .focused
            .filter(|focused| matches!(tree.tiles.get(*focused), Some(Tile::Pane(_))))
            .unwrap_or(default_focus);
        tree.make_active(|tile_id, _| tile_id == focused);
        if !tree.active_tiles().contains(&focused) {
            focused = default_focus;
            tree.make_active(|tile_id, _| tile_id == focused);
        }

        let layout = Self {
            tree,
            focused: Some(focused),
            selector_to_open: None,
            tab_to_reveal: None,
            focus_dirty: false,
        };
        request_pane_focus(egui_context, focused);
        Ok(layout)
    }
}
