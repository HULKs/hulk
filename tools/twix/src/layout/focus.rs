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
