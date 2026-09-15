use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use color_eyre::{
    Result,
    eyre::{ContextCompat, WrapErr as _, bail, ensure},
};
use eframe::{Storage, egui::Context};
use egui_tiles::{Container, Tile, TileId, Tiles, Tree};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SelectablePanel, backend::RobotBackend, panel::Panel, panels::TextPanel};

use super::{
    TwixLayout,
    tree::{active_pane_in_tile, first_pane},
    tree_id,
};

const MAX_RESTORED_TILE_ID: u64 = 1 << 20;
const MAX_RESTORED_TILE_COUNT: usize = 4096;
const MAX_RESTORED_TREE_DEPTH: usize = 128;

#[derive(Serialize, Deserialize)]
pub(super) struct LoadedLayout {
    tree: Tree<Value>,
    focused: Option<TileId>,
    #[serde(default)]
    names: HashMap<TileId, String>,
}

impl LoadedLayout {
    pub(super) fn parse(serialized: &str) -> Result<Self> {
        let loaded: Self =
            serde_json::from_str(serialized).wrap_err("failed to deserialize tile layout")?;
        validate_tree(&loaded.tree)?;
        Ok(loaded)
    }

    pub(super) fn blank() -> Self {
        Self {
            tree: Tree::new_tabs(
                "saved-layout",
                vec![serde_json::json!({"kind": TextPanel::STORAGE_ID, "state": {}})],
            ),
            focused: None,
            names: HashMap::new(),
        }
    }
}

impl TwixLayout {
    pub fn load(
        storage: Option<&dyn Storage>,
        context: &Context,
        backend: &Arc<RobotBackend>,
        clear: bool,
    ) -> Self {
        let mut layout = Self::new_session(context, backend);
        if !clear
            && let Some(saved) = storage.and_then(|storage| storage.get_string("workspace_session"))
        {
            match Self::from_serialized(&saved, backend, context) {
                Ok(restored) => layout = restored,
                Err(error) => {
                    layout.recovery = Some(saved);
                    layout.preset_ui.error = Some(format!(
                        "Could not restore session: {error:#}. Original saved in workspace_session_recovery."
                    ));
                }
            }
        }
        layout.activate(context);
        layout
    }

    pub fn save(&mut self, storage: &mut dyn Storage) {
        if let Some(saved) = &self.recovery {
            storage.set_string("workspace_session_recovery", saved.clone());
        }
        match self.snapshot() {
            Ok(layout) => storage.set_string("workspace_session", layout.to_string()),
            Err(error) => self.preset_ui.error = Some(format!("Could not save session: {error:#}")),
        }
    }

    pub(super) fn new_session(context: &Context, backend: &Arc<RobotBackend>) -> Self {
        let mut layout = Self::new(context, backend);
        let root = layout.tree.root.unwrap();
        layout.names.insert(root, "Workspace".into());
        layout.tree.root = Some(layout.tree.tiles.insert_tab_tile(vec![root]));
        layout
    }

    pub(super) fn title(&self, tile: TileId) -> String {
        self.names
            .get(&tile)
            .cloned()
            .unwrap_or_else(|| super::tab_bar::tab_title(&self.tree.tiles, tile))
    }

    pub fn snapshot(&self) -> Result<Value> {
        self.snapshot_subtree(self.tree.root.wrap_err("layout is empty")?)
    }

    pub(super) fn snapshot_subtree(&self, root: TileId) -> Result<Value> {
        let mut tiles = Tiles::default();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            let tile = match self.tree.tiles.get(id).wrap_err("missing tile")? {
                Tile::Pane(panel) => Tile::Pane(panel.save()),
                Tile::Container(container) => {
                    pending.extend(container.children().copied());
                    let mut container = container.clone();
                    if let Container::Tabs(tabs) = &mut container {
                        tabs.ensure_active(&self.tree.tiles);
                    }
                    Tile::Container(container)
                }
            };
            tiles.insert(id, tile);
            tiles.set_visible(id, self.tree.tiles.is_visible(id));
        }
        tiles.recompute_next_tile_id();
        let focused = self
            .focused
            .filter(|id| tiles.get(*id).is_some())
            .or_else(|| active_pane_in_tile(&self.tree.tiles, root));
        let names = self
            .names
            .iter()
            .filter(|(id, _)| tiles.get(**id).is_some())
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        Ok(serde_json::to_value(LoadedLayout {
            tree: Tree::new("saved-layout", root, tiles),
            focused,
            names,
        })?)
    }

    pub(super) fn insert_layout(
        &mut self,
        tabs: TileId,
        other: LoadedLayout,
        title: String,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) -> Result<TileId> {
        ensure!(
            matches!(
                self.tree.tiles.get(tabs),
                Some(Tile::Container(Container::Tabs(_)))
            ),
            "target is not a tab group"
        );
        ensure!(
            self.tree.tiles.len() + other.tree.tiles.len() <= MAX_RESTORED_TILE_COUNT,
            "too many tiles"
        );
        let root = other.tree.root.wrap_err("imported layout is empty")?;
        // Fresh IDs lie above both ranges, so replace_child cannot remap an already-remapped child.
        let start = (other
            .tree
            .tiles
            .tile_ids()
            .map(|id| id.0)
            .max()
            .unwrap_or(0)
            + 1)
        .max(self.tree.tiles.next_free_id().0);
        ensure!(
            start + other.tree.tiles.len() as u64 <= MAX_RESTORED_TILE_ID,
            "too many tile IDs"
        );
        let mut parent = Some(tabs);
        let mut depth = 0;
        while let Some(id) = parent {
            depth += 1;
            parent = self.tree.tiles.parent_of(id);
        }
        let mut pending = vec![(root, depth)];
        while let Some((id, depth)) = pending.pop() {
            ensure!(
                depth <= MAX_RESTORED_TREE_DEPTH,
                "layout exceeds maximum depth"
            );
            if let Some(Tile::Container(container)) = other.tree.tiles.get(id) {
                pending.extend(container.children().map(|id| (*id, depth + 1)));
            }
        }
        let mut other = Self::from_loaded(other, backend, egui_context)?;
        let ids: HashMap<_, _> = other
            .tree
            .tiles
            .tile_ids()
            .enumerate()
            .map(|(index, id)| (id, TileId(start + index as u64)))
            .collect();
        for (&old, &new) in &ids {
            let visible = other.tree.tiles.is_visible(old);
            let mut tile = other.tree.tiles.remove(old).unwrap();
            if let Tile::Container(container) = &mut tile {
                for child in container.children_vec() {
                    let _ = container.replace_child(child, ids[&child]);
                }
            }
            self.tree.tiles.insert(new, tile);
            self.tree.tiles.set_visible(new, visible);
        }
        self.tree.tiles.recompute_next_tile_id();
        self.names
            .extend(other.names.into_iter().map(|(id, name)| (ids[&id], name)));
        let root = ids[&root];
        self.names.insert(root, title);
        let Some(Tile::Container(Container::Tabs(target))) = self.tree.tiles.get_mut(tabs) else {
            unreachable!()
        };
        target.add_child(root);
        target.set_active(root);
        self.tree.make_active(|id, _| id == root);
        self.focused = other.focused.map(|id| ids[&id]);
        self.tab_to_reveal = Some(root);
        Ok(root)
    }

    pub fn validate(serialized: &str) -> Result<()> {
        LoadedLayout::parse(serialized)?;
        Ok(())
    }

    pub fn from_serialized(
        serialized: &str,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) -> Result<Self> {
        Self::from_loaded(LoadedLayout::parse(serialized)?, backend, egui_context)
    }

    fn from_loaded(
        loaded: LoadedLayout,
        backend: &Arc<RobotBackend>,
        egui_context: &Context,
    ) -> Result<Self> {
        let root = loaded.tree.root.wrap_err("tile layout has no root")?;
        let mut names = loaded.names;
        names.retain(|id, _| loaded.tree.tiles.get(*id).is_some());

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

        let mut tree = Tree::new(tree_id(), root, tiles);
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
            names,
            preset_ui: Default::default(),
            recovery: None,
            focused: Some(focused),
            selector_to_open: None,
            tab_to_reveal: None,
            focus_dirty: false,
        };
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
            match container {
                Container::Tabs(tabs) => ensure!(
                    tabs.active
                        .is_some_and(|active| tabs.children.contains(&active)),
                    "tile {tile_id:?} has no valid active tab"
                ),
                Container::Linear(linear) => {
                    let mut shares = linear.children.iter().map(|id| linear.shares[*id]);
                    let total: f32 = shares.clone().sum();
                    ensure!(
                        shares.all(|share| share.is_finite() && share >= 0.0)
                            && total.is_finite()
                            && total > 0.0,
                        "invalid split shares in {tile_id:?}"
                    );
                }
                Container::Grid(grid) => {
                    if let egui_tiles::GridLayout::Columns(columns) = grid.layout {
                        ensure!(
                            columns > 0 && columns <= MAX_RESTORED_TILE_COUNT,
                            "invalid grid columns"
                        );
                    }
                    for shares in [&grid.col_shares, &grid.row_shares] {
                        let total: f32 = shares.iter().sum();
                        ensure!(
                            shares.len() <= MAX_RESTORED_TILE_COUNT
                                && shares
                                    .iter()
                                    .all(|share| share.is_finite() && *share >= 0.0)
                                && (shares.is_empty() || (total.is_finite() && total > 0.0)),
                            "invalid grid shares"
                        );
                    }
                }
            }
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

    ensure!(
        seen.len() == tree.tiles.len(),
        "layout contains unreachable tiles"
    );
    ensure!(first_pane(tree).is_some(), "layout has no active panels");
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn malformed_layouts_fail_before_panel_construction() {
        let preset: Value = serde_json::from_str(crate::presets::PROVIDED[0].1).unwrap();
        for (pointer, value) in [
            ("/tree/root", json!(999)),
            (
                "/tree/tiles/tiles/5/Container/Linear/children",
                json!([2, 2]),
            ),
            ("/tree/tiles/tiles/5/Container/Linear/children", json!([5])),
            ("/tree/tiles/tiles/5/Container/Linear/children", json!([2])),
            ("/tree/tiles/tiles/2/Container/Tabs/active", json!(999)),
            (
                "/tree/tiles/tiles/5/Container/Linear/shares/shares",
                json!({"2": -1}),
            ),
            (
                "/tree/tiles/tiles/5/Container/Linear/shares/shares",
                json!({"2": 0, "4": 0}),
            ),
            ("/tree/tiles/invisible", json!([5])),
        ] {
            let mut bad = preset.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(
                TwixLayout::validate(&bad.to_string()).is_err(),
                "accepted {bad}"
            );
        }
    }

    #[test]
    fn layout_limits_are_enforced() {
        let mut tiles = Tiles::default();
        let mut root = tiles.insert_pane(json!({"kind": "text", "state": {}}));
        for _ in 0..=MAX_RESTORED_TREE_DEPTH {
            root = tiles.insert_tab_tile(vec![root]);
        }
        assert!(
            validate_tree(&Tree::new("deep", root, tiles))
                .unwrap_err()
                .to_string()
                .contains("depth")
        );
        let mut tiles = Tiles::default();
        let root = TileId(MAX_RESTORED_TILE_ID + 1);
        tiles.insert(root, Tile::Pane(json!({"kind": "text", "state": {}})));
        assert!(validate_tree(&Tree::new("large-id", root, tiles)).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn presets_render_round_trip_and_isolate_focus() {
        use eframe::egui::{CentralPanel, RawInput, Rect, vec2};
        let backend = Arc::new(
            RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
                .await
                .unwrap(),
        );
        let context = Context::default();
        egui_material_icons::initialize(&context);
        for &(name, preset) in crate::presets::PROVIDED {
            let mut layout = TwixLayout::from_serialized(preset, &backend, &context).unwrap();
            let snapshot = layout.snapshot().unwrap();
            let mut copy =
                TwixLayout::from_serialized(&snapshot.to_string(), &backend, &context).unwrap();
            assert_ne!(layout.tree.id(), copy.tree.id());
            let copied = copy.snapshot().unwrap();
            assert_eq!(snapshot, copied, "{name}");
            for width in [320.0, 1280.0] {
                for layout in [&mut layout, &mut copy] {
                    layout.activate(&context);
                    let focused = layout.focused.unwrap();
                    assert!(context.memory(|memory| memory.has_focus(
                        super::super::focus::pane_focus_id(layout.tree.id(), focused)
                    )));
                    for _ in 0..3 {
                        let _ = context.run_ui(
                            RawInput {
                                screen_rect: Some(Rect::from_min_size(
                                    Default::default(),
                                    vec2(width, 720.0),
                                )),
                                ..Default::default()
                            },
                            |ui| {
                                CentralPanel::default().show(ui, |ui| layout.ui(ui, &backend));
                            },
                        );
                    }
                    assert!(
                        layout.tree.tiles.rect(focused).unwrap().is_positive(),
                        "{name}"
                    );
                    TwixLayout::validate(&layout.snapshot().unwrap().to_string()).unwrap();
                }
            }
        }
    }
}
