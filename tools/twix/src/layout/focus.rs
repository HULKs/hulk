use std::cmp::Ordering;

use eframe::egui::{Context, FocusDirection as EguiFocusDirection, Id, Rangef, Rect};
use egui_tiles::{Tile, TileId};

use super::{TREE_ID, TwixLayout};

#[derive(Clone, Copy)]
pub enum FocusDirection {
    Left,
    Right,
    Above,
    Below,
}

impl TwixLayout {
    pub fn focus_topic(&mut self, egui_context: &Context) {
        self.repair_focus(egui_context);
        if let Some(focused) = self.focused
            && let Some(Tile::Pane(panel)) = self.tree.tiles.get_mut(focused)
        {
            panel.focus_topic();
        }
    }

    pub fn focus(&mut self, direction: FocusDirection, egui_context: &Context) {
        let Some(current_id) = self.focused else {
            return;
        };
        let Some(current_rect) = self.tree.tiles.rect(current_id) else {
            return;
        };
        let candidates = self
            .tree
            .active_tiles()
            .into_iter()
            .filter(|&tile_id| tile_id != current_id)
            .filter_map(|tile_id| match self.tree.tiles.get(tile_id) {
                Some(Tile::Pane(_)) => self.tree.tiles.rect(tile_id).map(|rect| (tile_id, rect)),
                _ => None,
            });
        let Some(next) = nearest_in_direction(current_rect, candidates, direction) else {
            return;
        };

        egui_context.memory_mut(|memory| memory.move_focus(EguiFocusDirection::None));
        self.set_focus(next, egui_context);
    }
}

pub(super) fn pane_focus_id(tile_id: TileId) -> Id {
    Id::new(TREE_ID).with(("pane-focus", tile_id))
}

pub(super) fn request_pane_focus(egui_context: &Context, tile_id: TileId) {
    egui_context.memory_mut(|memory| memory.request_focus(pane_focus_id(tile_id)));
}

pub(super) fn nearest_in_direction(
    current: Rect,
    candidates: impl Iterator<Item = (TileId, Rect)>,
    direction: FocusDirection,
) -> Option<TileId> {
    candidates
        .filter_map(|(tile_id, candidate)| {
            direction_score(current, candidate, direction).map(|score| (tile_id, score))
        })
        .min_by(|(_, left), (_, right)| compare_scores(*left, *right))
        .map(|(tile_id, _)| tile_id)
}

fn direction_score(
    current: Rect,
    candidate: Rect,
    direction: FocusDirection,
) -> Option<(bool, f32, f32, f32)> {
    let current_center = current.center();
    let candidate_center = candidate.center();
    let (in_direction, primary_gap, orthogonal_gap, orthogonal_center_distance) = match direction {
        FocusDirection::Left => (
            candidate_center.x < current_center.x,
            (current.left() - candidate.right()).max(0.0),
            range_gap(current.y_range(), candidate.y_range()),
            (current_center.y - candidate_center.y).abs(),
        ),
        FocusDirection::Right => (
            current_center.x < candidate_center.x,
            (candidate.left() - current.right()).max(0.0),
            range_gap(current.y_range(), candidate.y_range()),
            (current_center.y - candidate_center.y).abs(),
        ),
        FocusDirection::Above => (
            candidate_center.y < current_center.y,
            (current.top() - candidate.bottom()).max(0.0),
            range_gap(current.x_range(), candidate.x_range()),
            (current_center.x - candidate_center.x).abs(),
        ),
        FocusDirection::Below => (
            current_center.y < candidate_center.y,
            (candidate.top() - current.bottom()).max(0.0),
            range_gap(current.x_range(), candidate.x_range()),
            (current_center.x - candidate_center.x).abs(),
        ),
    };
    in_direction.then_some((
        0.0 < orthogonal_gap,
        primary_gap,
        orthogonal_gap,
        orthogonal_center_distance,
    ))
}

fn range_gap(current: Rangef, candidate: Rangef) -> f32 {
    (current.min - candidate.max)
        .max(candidate.min - current.max)
        .max(0.0)
}

fn compare_scores(left: (bool, f32, f32, f32), right: (bool, f32, f32, f32)) -> Ordering {
    left.0
        .cmp(&right.0)
        .then_with(|| left.1.total_cmp(&right.1))
        .then_with(|| left.2.total_cmp(&right.2))
        .then_with(|| left.3.total_cmp(&right.3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        PanelKind,
        backend::RobotBackend,
        configuration::{
            Configuration,
            keybind_plugin::{self, KeybindSystem},
            keys::KeybindAction,
        },
        layout::tree::LayoutRequest,
    };
    use eframe::egui::{
        CentralPanel, Event, Key, Modifiers, Pos2, RawInput, Rect, text_edit::TextEditState, vec2,
    };
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn topic_shortcut_targets_the_active_panel_across_splits_and_panel_types() {
        fn frame(
            context: &Context,
            backend: &Arc<RobotBackend>,
            layout: &mut TwixLayout,
            shortcut: bool,
        ) {
            let events = if shortcut {
                vec![Event::Key {
                    key: Key::F,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                }]
            } else {
                vec![]
            };
            let _ = context.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if context.keybind_pressed(KeybindAction::FocusTopic) {
                        layout.focus_topic(context);
                    }
                    CentralPanel::default().show(ui, |ui| layout.ui(ui, backend));
                },
            );
        }

        let backend = Arc::new(
            RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
                .await
                .unwrap(),
        );
        let context = Context::default();
        let config: Configuration = toml::from_str("[keys]\nC-f = \"focus_topic\"\n").unwrap();
        keybind_plugin::register(&context);
        context.set_keybinds(Arc::new(config.keys));
        let mut layout = TwixLayout::new(&context, &backend);
        let left = layout.focused.unwrap();
        layout.open_split(&backend, &context);
        let right = layout.focused.unwrap();
        frame(&context, &backend, &mut layout, false);
        for (pane, kind) in [
            (left, PanelKind::TextPanel),
            (right, PanelKind::TextPanel),
            (right, PanelKind::ImagePanel),
        ] {
            layout.apply_request(
                LayoutRequest::Replace { pane, panel: kind },
                &backend,
                &context,
            );
            layout.set_focus(pane, &context);
            frame(&context, &backend, &mut layout, false);
            frame(&context, &backend, &mut layout, true);
            let focused = context
                .memory(|memory| memory.focused())
                .expect("topic input should have focus");
            assert!(
                TextEditState::load(&context, focused).is_some(),
                "focused widget should be a text input"
            );
            let response = context.read_response(focused).unwrap();
            assert!(
                layout
                    .tree
                    .tiles
                    .rect(pane)
                    .unwrap()
                    .contains(response.rect.center()),
                "another panel stole topic focus"
            );
            frame(&context, &backend, &mut layout, false);
            assert_eq!(
                context.memory(|memory| memory.focused()),
                Some(focused),
                "focus should persist"
            );
        }
        layout.apply_request(
            LayoutRequest::Replace {
                pane: right,
                panel: PanelKind::ParameterPanel,
            },
            &backend,
            &context,
        );
        layout.set_focus(right, &context);
        frame(&context, &backend, &mut layout, false);
        frame(&context, &backend, &mut layout, true);
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(pane_focus_id(right)),
            "panels without topics should leave focus alone"
        );
    }
}
