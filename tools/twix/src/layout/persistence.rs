use std::{collections::HashSet, sync::Arc};

use color_eyre::{
    Result,
    eyre::{ContextCompat, WrapErr as _, bail, ensure},
};
use eframe::{CreationContext, Storage, egui::Context};
use egui_tiles::{Tile, TileId, Tiles, Tree};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SelectablePanel, backend::RobotBackend};

use super::{TREE_ID, TwixLayout, focus::request_pane_focus, tree::first_pane};

const STORAGE_KEY: &str = "tile_layout";
const MAX_RESTORED_TILE_ID: u64 = 1 << 20;
const MAX_RESTORED_TILE_COUNT: usize = 4096;
const MAX_RESTORED_TREE_DEPTH: usize = 128;

#[derive(Serialize)]
pub(super) struct SavedLayout<'a> {
    pub(super) tree: &'a Tree<SelectablePanel>,
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
        let root = validate_tree(&loaded.tree)?;

        let mut tiles = Tiles::default();
        for (tile_id, tile) in loaded.tree.tiles.iter() {
            let tile = match tile {
                Tile::Pane(value) => Tile::Pane(
                    SelectablePanel::restore(backend, value, egui_context).unwrap_or_else(
                        |error| {
                            error!("failed to restore panel in tile {tile_id:?}: {error:#}");
                            SelectablePanel::text(backend, egui_context)
                        },
                    ),
                ),
                Tile::Container(container) => Tile::Container(container.clone()),
            };
            tiles.insert(*tile_id, tile);
            tiles.set_visible(*tile_id, loaded.tree.tiles.is_visible(*tile_id));
        }
        tiles.recompute_next_tile_id();

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

fn validate_tree(tree: &Tree<Value>) -> Result<TileId> {
    ensure!(
        tree.tiles.len() <= MAX_RESTORED_TILE_COUNT,
        "tile layout contains too many tiles"
    );
    ensure!(
        tree.tiles
            .tile_ids()
            .all(|tile_id| tile_id.0 <= MAX_RESTORED_TILE_ID),
        "tile layout contains an unsupported tile ID"
    );
    let root = tree.root.wrap_err("tile layout has no root")?;
    let mut seen = HashSet::new();
    let mut stack = vec![(root, 0_usize)];

    while let Some((tile_id, depth)) = stack.pop() {
        ensure!(
            depth <= MAX_RESTORED_TREE_DEPTH,
            "tile layout exceeds the maximum supported depth"
        );
        if !seen.insert(tile_id) {
            bail!("tile {tile_id:?} is referenced more than once");
        }
        let Some(tile) = tree.tiles.get(tile_id) else {
            bail!("tile layout references missing tile {tile_id:?}");
        };
        if let Tile::Container(container) = tile {
            ensure!(
                seen.len() + stack.len() + container.num_children() <= MAX_RESTORED_TILE_COUNT,
                "tile layout contains too many child references"
            );
            stack.extend(
                container
                    .children()
                    .copied()
                    .map(|child| (child, depth + 1)),
            );
        }
    }

    Ok(root)
}
