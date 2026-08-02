use std::sync::Arc;

use eframe::egui::{
    CentralPanel, Context, Event, Id, Key, Modifiers, PointerButton, Popup, Pos2, RawInput, Rect,
    pos2, vec2,
};
use egui_tiles::{Container, Linear, LinearDir, Tile, TileId, Tiles, Tree};

use crate::{PanelKind, backend::RobotBackend};

use super::{
    FocusDirection, TREE_ID, TwixLayout,
    focus::{nearest_in_direction, pane_focus_id},
    pane::PanelPane,
    persistence::{MAX_RESTORED_TILE_ID, SavedLayout},
    tab_bar::tab_title,
    tree::{LayoutRequest, TabGroupId, simplification_options},
};

async fn test_backend() -> Arc<RobotBackend> {
    Arc::new(
        RobotBackend::new(tokio::runtime::Handle::current(), None, "/".to_string())
            .await
            .expect("backend should build"),
    )
}

async fn test_layout() -> (Arc<RobotBackend>, Context, TwixLayout) {
    let backend = test_backend().await;
    let context = Context::default();
    let layout = TwixLayout::new(&context, &backend);
    (backend, context, layout)
}

fn pane_count(layout: &TwixLayout) -> usize {
    layout
        .tree
        .tiles
        .tiles()
        .filter(|tile| matches!(tile, Tile::Pane(_)))
        .count()
}

fn show_layout(
    context: &Context,
    layout: &mut TwixLayout,
    backend: &Arc<RobotBackend>,
    events: Vec<Event>,
) {
    show_layout_at_width(context, layout, backend, 800.0, events);
}

fn show_layout_at_width(
    context: &Context,
    layout: &mut TwixLayout,
    backend: &Arc<RobotBackend>,
    width: f32,
    events: Vec<Event>,
) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 600.0))),
        events,
        ..Default::default()
    };
    let _output = context.run_ui(input, |ui| {
        CentralPanel::default().show(ui, |ui| layout.ui(ui, backend));
    });
}

fn click_layout(
    context: &Context,
    layout: &mut TwixLayout,
    backend: &Arc<RobotBackend>,
    position: Pos2,
) {
    show_layout(
        context,
        layout,
        backend,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    show_layout(
        context,
        layout,
        backend,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
}

fn tab_id(tile_id: TileId) -> Id {
    tile_id.egui_id(Id::new(TREE_ID))
}

fn add_grouped_split(
    layout: &mut TwixLayout,
    backend: &Arc<RobotBackend>,
    context: &Context,
) -> (TileId, TileId, TileId) {
    let sibling = layout.focused.expect("default panel should be focused");
    layout.open_tab(backend, context);
    let group = layout.focused.expect("new panel should be focused");
    let original = layout
        .tree
        .tiles
        .remove(group)
        .expect("grouped panel should exist");
    let original = layout.tree.tiles.insert_new(original);
    let original_tabs = layout.tree.tiles.insert_tab_tile(vec![original]);
    let added = layout
        .tree
        .tiles
        .insert_pane(PanelPane::text(backend, context));
    let added_tabs = layout.tree.tiles.insert_tab_tile(vec![added]);
    layout.tree.tiles.insert(
        group,
        Tile::Container(Container::Linear(Linear::new_binary(
            LinearDir::Horizontal,
            [original_tabs, added_tabs],
            0.5,
        ))),
    );
    layout.set_focus(original, context);

    (group, sibling, original)
}

fn round_trip_layout(
    layout: &TwixLayout,
    backend: &Arc<RobotBackend>,
    context: &Context,
) -> TwixLayout {
    let serialized = serde_json::to_string(&SavedLayout {
        tree: &layout.tree,
        focused: layout.focused,
    })
    .expect("layout should serialize");
    TwixLayout::from_serialized(&serialized, backend, context).expect("layout should restore")
}

async fn assert_invalid_tree(tree: Tree<serde_json::Value>) {
    let backend = test_backend().await;
    let context = Context::default();
    let serialized = serde_json::to_string(&serde_json::json!({
        "tree": tree,
        "focused": null,
    }))
    .expect("layout should serialize");
    assert!(TwixLayout::from_serialized(&serialized, &backend, &context).is_err());
}

#[test]
fn directional_focus_prefers_orthogonally_overlapping_pane() {
    let current = Rect::from_min_max(pos2(100.0, 100.0), pos2(200.0, 200.0));
    let aligned = (
        TileId::from_u64(1),
        Rect::from_min_max(pos2(0.0, 100.0), pos2(90.0, 200.0)),
    );
    let diagonal = (
        TileId::from_u64(2),
        Rect::from_min_max(pos2(50.0, 0.0), pos2(90.0, 90.0)),
    );

    assert_eq!(
        nearest_in_direction(
            current,
            [diagonal, aligned].into_iter(),
            FocusDirection::Left
        ),
        Some(aligned.0)
    );
}

#[test]
fn directional_focus_ignores_panes_behind_requested_direction() {
    let current = Rect::from_min_max(pos2(100.0, 100.0), pos2(200.0, 200.0));
    let left = (
        TileId::from_u64(1),
        Rect::from_min_max(pos2(0.0, 100.0), pos2(90.0, 200.0)),
    );

    assert_eq!(
        nearest_in_direction(current, [left].into_iter(), FocusDirection::Right),
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn tab_and_split_commands_keep_every_pane_in_a_tab_group() {
    let (backend, context, mut layout) = test_layout().await;

    layout.open_tab(&backend, &context);
    assert_eq!(pane_count(&layout), 2);
    let tabs = layout
        .focused_tabs()
        .expect("focused pane should have tabs");
    assert!(matches!(
        layout.tree.tiles.get(tabs.0),
        Some(Tile::Container(Container::Tabs(tabs))) if tabs.children.len() == 2
    ));

    layout.open_split(&backend, &context);
    assert_eq!(pane_count(&layout), 3);
    assert!(layout.focused_tabs().is_some());
    assert!(matches!(
        layout
            .tree
            .root()
            .and_then(|root| layout.tree.tiles.get(root)),
        Some(Tile::Container(Container::Linear(_)))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restored_layout_wraps_a_pane_dropped_between_splits_without_reusing_its_id() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    layout.open_split(&backend, &context);
    let mut restored = round_trip_layout(&layout, &backend, &context);
    let root = restored.tree.root().expect("layout should have a root");
    let moved = restored
        .tree
        .tiles
        .iter()
        .find_map(|(_, tile)| match tile {
            Tile::Container(Container::Tabs(tabs)) if tabs.children.len() == 2 => {
                tabs.children.first().copied()
            }
            _ => None,
        })
        .expect("one split should contain two tabs");

    restored.tree.move_tile_to_container(moved, root, 1, false);
    restored.tree.simplify(&simplification_options());

    let wrapped_pane = match restored.tree.tiles.get(moved) {
        Some(Tile::Container(Container::Tabs(tabs))) => {
            assert_eq!(tabs.children.len(), 1);
            tabs.children[0]
        }
        _ => panic!("direct pane should be wrapped in tabs"),
    };
    assert_ne!(wrapped_pane, moved);
    assert!(matches!(
        restored.tree.tiles.get(wrapped_pane),
        Some(Tile::Pane(_))
    ));
    assert!(!restored.tree.active_tiles().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn closing_an_inactive_tab_preserves_focus() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    layout.open_tab(&backend, &context);
    let tabs_id = layout
        .focused_tabs()
        .expect("focused pane should have tabs");
    let children = match layout.tree.tiles.get(tabs_id.0) {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children.clone(),
        _ => panic!("focused pane should be in tabs"),
    };
    let focused = *children.last().expect("tabs should contain panels");

    layout.close_tile(children[1], &backend, &context);

    assert_eq!(layout.focused, Some(focused));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn closing_the_first_focused_tab_focuses_its_sibling() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    let tabs_id = layout
        .focused_tabs()
        .expect("focused pane should have tabs");
    let children = match layout.tree.tiles.get(tabs_id.0) {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children.clone(),
        _ => panic!("focused pane should be in tabs"),
    };
    layout.open_split(&backend, &context);
    layout.set_focus(children[0], &context);

    layout.close_tile(children[0], &backend, &context);

    assert_eq!(layout.focused, Some(children[1]));
    assert!(context.memory(|memory| memory.has_focus(pane_focus_id(children[1]))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn empty_tile_layout_is_rejected() {
    assert_invalid_tree(Tree::empty("empty")).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn implausibly_large_persisted_tile_id_is_rejected() {
    let pane_id = TileId::from_u64(MAX_RESTORED_TILE_ID + 1);
    let mut tiles = egui_tiles::Tiles::default();
    tiles.insert(
        pane_id,
        Tile::Pane(serde_json::json!({ "kind": "text", "state": {} })),
    );
    assert_invalid_tree(Tree::new("oversized", pane_id, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persisted_layout_with_a_missing_child_is_rejected() {
    let root = TileId::from_u64(1);
    let missing = TileId::from_u64(2);
    let mut tiles = Tiles::default();
    tiles.insert(root, Tile::Container(Container::new_tabs(vec![missing])));

    assert_invalid_tree(Tree::new("missing-child", root, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persisted_layout_with_an_orphan_is_rejected() {
    let mut tree = Tree::new_tabs(
        "orphan",
        vec![serde_json::json!({ "kind": "text", "state": {} })],
    );
    let _ = tree.tiles.insert_pane(serde_json::json!({
        "kind": "text",
        "state": {}
    }));
    assert_invalid_tree(tree).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persisted_layout_with_an_invalid_active_tab_is_rejected() {
    let pane = TileId::from_u64(1);
    let root = TileId::from_u64(2);
    let mut tiles = Tiles::default();
    tiles.insert(
        pane,
        Tile::Pane(serde_json::json!({ "kind": "text", "state": {} })),
    );
    let mut tabs = egui_tiles::Tabs::new(vec![pane]);
    tabs.active = Some(TileId::from_u64(3));
    tiles.insert(root, Tile::Container(Container::Tabs(tabs)));
    assert_invalid_tree(Tree::new("invalid-active", root, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persisted_layout_with_a_grid_is_rejected() {
    let pane = TileId::from_u64(1);
    let root = TileId::from_u64(2);
    let mut tiles = Tiles::default();
    tiles.insert(
        pane,
        Tile::Pane(serde_json::json!({ "kind": "text", "state": {} })),
    );
    tiles.insert(root, Tile::Container(Container::new_grid(vec![pane])));
    assert_invalid_tree(Tree::new("grid", root, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persisted_layout_with_a_multiply_parented_pane_is_rejected() {
    let pane = TileId::from_u64(1);
    let left = TileId::from_u64(2);
    let right = TileId::from_u64(3);
    let root = TileId::from_u64(4);
    let mut tiles = Tiles::default();
    tiles.insert(
        pane,
        Tile::Pane(serde_json::json!({ "kind": "text", "state": {} })),
    );
    tiles.insert(left, Tile::Container(Container::new_tabs(vec![pane])));
    tiles.insert(right, Tile::Container(Container::new_tabs(vec![pane])));
    tiles.insert(
        root,
        Tile::Container(Container::new_horizontal(vec![left, right])),
    );
    assert_invalid_tree(Tree::new("multiple-parents", root, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn excessively_deep_persisted_layout_is_rejected() {
    let pane = TileId::from_u64(1);
    let mut tiles = Tiles::default();
    tiles.insert(
        pane,
        Tile::Pane(serde_json::json!({ "kind": "text", "state": {} })),
    );
    let mut child = pane;
    for raw_id in 2..=132 {
        let parent = TileId::from_u64(raw_id);
        tiles.insert(
            parent,
            Tile::Container(Container::new_horizontal(vec![child])),
        );
        child = parent;
    }
    assert_invalid_tree(Tree::new("too-deep", child, tiles)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn failed_panel_restoration_rejects_the_entire_layout() {
    let tree = Tree::new_tabs(
        "unknown-panel",
        vec![serde_json::json!({
            "kind": "not-a-registered-panel",
            "state": {}
        })],
    );
    assert_invalid_tree(tree).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restored_layout_preserves_tile_visibility() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    let tabs_id = layout.focused_tabs().expect("layout should contain tabs");
    let hidden = match layout.tree.tiles.get(tabs_id.0) {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children[0],
        _ => panic!("focused pane should be in tabs"),
    };
    layout.tree.tiles.set_visible(hidden, false);
    let restored = round_trip_layout(&layout, &backend, &context);

    assert!(!restored.tree.tiles.is_visible(hidden));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn stale_add_request_does_not_create_an_orphaned_pane() {
    let (backend, context, mut layout) = test_layout().await;
    let original_count = pane_count(&layout);

    layout.apply_request(
        LayoutRequest::Add {
            tabs: TabGroupId(TileId::from_u64(999)),
            panel: PanelKind::registered()[0],
        },
        &backend,
        &context,
    );

    assert_eq!(pane_count(&layout), original_count);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn newly_opened_tab_does_not_scroll_when_it_already_fits() {
    let (backend, context, mut layout) = test_layout().await;
    let first = layout.focused.expect("default panel should be focused");
    show_layout(&context, &mut layout, &backend, Vec::new());
    let first_left = context
        .read_response(tab_id(first))
        .expect("panel should have a tab response")
        .rect
        .left();

    layout.open_tab(&backend, &context);
    let opened = layout.focused.expect("new panel should be focused");
    assert_eq!(layout.tab_to_reveal, Some(opened));

    show_layout(&context, &mut layout, &backend, Vec::new());
    assert_eq!(layout.tab_to_reveal, Some(opened));
    assert_eq!(
        context
            .read_response(tab_id(first))
            .expect("panel should retain its tab response")
            .rect
            .left(),
        first_left
    );
    show_layout(&context, &mut layout, &backend, Vec::new());
    assert_eq!(layout.tab_to_reveal, None);
    assert_eq!(
        context
            .read_response(tab_id(first))
            .expect("panel should retain its tab response")
            .rect
            .left(),
        first_left
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn newly_opened_clipped_tab_is_revealed() {
    let (backend, context, mut layout) = test_layout().await;
    for _ in 0..3 {
        layout.open_tab(&backend, &context);
    }
    let opened = layout.focused.expect("new panel should be focused");

    show_layout_at_width(&context, &mut layout, &backend, 240.0, Vec::new());
    let response = context
        .read_response(tab_id(opened))
        .expect("new panel should have a tab response");
    assert!(!response.interact_rect.contains_rect(response.rect));

    show_layout_at_width(&context, &mut layout, &backend, 240.0, Vec::new());
    assert_eq!(layout.tab_to_reveal, None);
    show_layout_at_width(&context, &mut layout, &backend, 240.0, Vec::new());
    let response = context
        .read_response(tab_id(opened))
        .expect("new panel should retain its tab response");
    assert!(response.interact_rect.contains_rect(response.rect));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn stale_selector_target_is_repaired_for_the_next_frame() {
    let (backend, context, mut layout) = test_layout().await;
    layout.selector_to_open = Some(TileId::from_u64(999));

    show_layout(&context, &mut layout, &backend, Vec::new());

    assert_eq!(layout.selector_to_open, layout.focused);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn panel_selector_only_opens_from_the_caret() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    show_layout(&context, &mut layout, &backend, Vec::new());
    show_layout(&context, &mut layout, &backend, Vec::new());
    let first = match layout
        .tree
        .root()
        .and_then(|root| layout.tree.tiles.get(root))
    {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children[0],
        _ => panic!("layout should contain tabs"),
    };
    let id = tab_id(first);
    let tab_rect = context
        .read_response(id)
        .expect("panel should have a tab response")
        .rect;
    let popup_id = id.with("panel-type-popup");
    let title_position = tab_rect.left_center() + vec2(8.0, 0.0);

    click_layout(&context, &mut layout, &backend, title_position);

    assert_eq!(layout.focused, Some(first));
    assert!(!Popup::is_id_open(&context, popup_id));
    let selector_rect = context
        .read_response(id.with("panel-type"))
        .expect("panel should retain its selector response")
        .rect;

    click_layout(&context, &mut layout, &backend, selector_rect.center());

    assert!(Popup::is_id_open(&context, popup_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn group_tabs_have_no_panel_selector_and_survive_persistence() {
    let (backend, context, mut layout) = test_layout().await;
    let (group, _, focused) = add_grouped_split(&mut layout, &backend, &context);

    show_layout(&context, &mut layout, &backend, Vec::new());

    assert!(context.read_response(tab_id(group)).is_some());
    assert!(
        context
            .read_response(tab_id(group).with("panel-type"))
            .is_none()
    );
    let restored = round_trip_layout(&layout, &backend, &context);

    assert_eq!(pane_count(&restored), 3);
    assert_eq!(restored.focused, Some(focused));
    assert!(restored.focused_tabs().is_some());
    assert_eq!(tab_title(&restored.tree.tiles, group), "Group");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn closing_a_group_closes_its_subtree_and_focuses_its_sibling() {
    let (backend, context, mut layout) = test_layout().await;
    let (group, sibling, _) = add_grouped_split(&mut layout, &backend, &context);

    layout.apply_request(LayoutRequest::Close(group), &backend, &context);

    assert_eq!(pane_count(&layout), 1);
    assert_eq!(layout.focused, Some(sibling));
    assert!(layout.tree.tiles.get(group).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn focus_repair_only_scans_the_tree_when_dirty() {
    let (backend, context, mut layout) = test_layout().await;
    show_layout(&context, &mut layout, &backend, Vec::new());
    let reachable = layout.focused.expect("default pane should be focused");
    let orphan = layout
        .tree
        .tiles
        .insert_pane(PanelPane::text(&backend, &context));
    layout.focused = Some(orphan);

    show_layout(&context, &mut layout, &backend, Vec::new());
    assert_eq!(layout.focused, Some(orphan));

    layout.focus_dirty = true;
    show_layout(&context, &mut layout, &backend, Vec::new());
    assert_eq!(layout.focused, Some(reachable));
    assert!(!layout.focus_dirty);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn closing_a_focused_tab_skips_hidden_fallback_siblings() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    layout.open_tab(&backend, &context);
    let tabs_id = layout.focused_tabs().expect("layout should contain tabs");
    let children = match layout.tree.tiles.get(tabs_id.0) {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children.clone(),
        _ => panic!("focused pane should be in tabs"),
    };
    layout.tree.tiles.set_visible(children[1], false);

    layout.close_tile(children[2], &backend, &context);

    assert_eq!(layout.focused, Some(children[0]));
    assert!(layout.tree.active_tiles().contains(&children[0]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn directional_focus_at_a_boundary_preserves_widget_focus() {
    let (backend, context, mut layout) = test_layout().await;
    show_layout(&context, &mut layout, &backend, Vec::new());
    let widget = Id::new("existing-widget-focus");
    context.memory_mut(|memory| memory.request_focus(widget));

    layout.focus(FocusDirection::Left, &context);

    assert!(context.memory(|memory| memory.has_focus(widget)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn moving_active_tab_between_splits_repairs_source_and_focus() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_split(&backend, &context);
    layout.open_tab(&backend, &context);
    let source_tabs = layout
        .focused_tabs()
        .expect("focused pane should have tabs");
    let (remaining, moved) = match layout.tree.tiles.get(source_tabs.0) {
        Some(Tile::Container(Container::Tabs(tabs))) => (tabs.children[0], tabs.children[1]),
        _ => panic!("focused pane should be in tabs"),
    };
    let root = layout.tree.root().expect("split layout should have a root");

    layout.tree.move_tile_to_container(moved, root, 1, false);

    assert!(matches!(
        layout.tree.tiles.get(source_tabs.0),
        Some(Tile::Container(Container::Tabs(tabs))) if tabs.is_active(remaining)
    ));
    layout.tree.simplify(&simplification_options());
    layout.repair_focus(&context);

    assert!(matches!(
        layout.tree.tiles.get(root),
        Some(Tile::Container(Container::Linear(linear))) if linear.children.len() == 3
    ));
    for tile in layout.tree.tiles.tiles() {
        if let Tile::Container(Container::Tabs(tabs)) = tile {
            assert!(
                tabs.active
                    .is_none_or(|active| tabs.children.contains(&active))
            );
        }
    }
    assert!(
        layout
            .focused
            .is_some_and(|focused| matches!(layout.tree.tiles.get(focused), Some(Tile::Pane(_))))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn selected_pane_has_a_full_sized_focusable_response() {
    let (backend, context, mut layout) = test_layout().await;
    let focused = layout.focused.expect("default layout should have a panel");

    show_layout(&context, &mut layout, &backend, Vec::new());

    let pane = context
        .read_response(pane_focus_id(focused))
        .expect("focused pane should have a response");
    let tile_rect = layout
        .tree
        .tiles
        .rect(focused)
        .expect("focused pane should have a rectangle");
    assert!(pane.sense.is_focusable());
    assert!(pane.rect.contains_rect(tile_rect));
    assert!(context.memory(|memory| memory.has_focus(pane.id)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dragging_a_tab_closes_an_open_panel_selector() {
    let (backend, context, mut layout) = test_layout().await;
    layout.open_tab(&backend, &context);
    show_layout(&context, &mut layout, &backend, Vec::new());
    let children = match layout
        .tree
        .root()
        .and_then(|root| layout.tree.tiles.get(root))
    {
        Some(Tile::Container(Container::Tabs(tabs))) => tabs.children.clone(),
        _ => panic!("layout should contain tabs"),
    };
    let popup_id = tab_id(children[0]).with("panel-type-popup");
    Popup::open_id(&context, popup_id);
    show_layout(&context, &mut layout, &backend, Vec::new());
    let dragged_rect = context
        .read_response(tab_id(children[1]))
        .expect("panel should have a tab response")
        .rect;
    let start = dragged_rect.left_center() + vec2(8.0, 0.0);

    show_layout(
        &context,
        &mut layout,
        &backend,
        vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    show_layout(
        &context,
        &mut layout,
        &backend,
        vec![Event::PointerMoved(start + vec2(20.0, 0.0))],
    );

    assert_eq!(layout.tree.dragged_id(&context), Some(children[1]));
    assert!(!Popup::is_id_open(&context, popup_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn tab_from_a_mouse_selected_pane_focuses_a_panel_control() {
    let (backend, context, mut layout) = test_layout().await;
    show_layout(&context, &mut layout, &backend, Vec::new());
    let focused = layout.focused.expect("default layout should have a panel");
    let pane_id = pane_focus_id(focused);
    let pane_rect = layout
        .tree
        .tiles
        .rect(focused)
        .expect("focused pane should have a rectangle");
    let unrelated = Id::new("unrelated-focus-target");
    context.memory_mut(|memory| memory.request_focus(unrelated));

    click_layout(
        &context,
        &mut layout,
        &backend,
        pane_rect.right_bottom() - vec2(8.0, 8.0),
    );

    assert!(context.memory(|memory| memory.has_focus(pane_id)));

    show_layout(
        &context,
        &mut layout,
        &backend,
        vec![Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );

    let focused_widget = context
        .memory(|memory| memory.focused())
        .expect("a panel control should receive focus");
    assert_ne!(focused_widget, pane_id);
    let focused_rect = context
        .read_response(focused_widget)
        .expect("focused control should have a response")
        .rect;
    assert!(pane_rect.contains(focused_rect.center()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn clicking_a_panel_control_does_not_return_focus_to_the_pane() {
    let (backend, context, mut layout) = test_layout().await;
    show_layout(&context, &mut layout, &backend, Vec::new());
    let focused = layout.focused.expect("default layout should have a panel");
    show_layout(
        &context,
        &mut layout,
        &backend,
        vec![Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let control = context
        .memory(|memory| memory.focused())
        .expect("a panel control should receive focus");
    let control_rect = context
        .read_response(control)
        .expect("focused control should have a response")
        .rect;
    context.memory_mut(|memory| memory.request_focus(Id::new("unrelated-control-focus")));

    click_layout(&context, &mut layout, &backend, control_rect.center());

    assert_eq!(layout.focused, Some(focused));
    assert!(context.memory(|memory| memory.has_focus(control)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn directional_selection_focuses_the_destination_pane() {
    let (backend, context, mut layout) = test_layout().await;
    let left = layout.focused.expect("default layout should have a panel");
    layout.open_split(&backend, &context);
    show_layout(&context, &mut layout, &backend, Vec::new());

    layout.focus(FocusDirection::Left, &context);

    assert_eq!(layout.focused, Some(left));
    assert!(context.memory(|memory| memory.has_focus(pane_focus_id(left))));

    show_layout(
        &context,
        &mut layout,
        &backend,
        vec![Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let focused_widget = context
        .memory(|memory| memory.focused())
        .expect("a panel control should receive focus");
    let left_rect = layout
        .tree
        .tiles
        .rect(left)
        .expect("left pane should have a rectangle");
    let focused_rect = context
        .read_response(focused_widget)
        .expect("focused control should have a response")
        .rect;
    assert_ne!(focused_widget, pane_focus_id(left));
    assert!(left_rect.contains(focused_rect.center()));
}
