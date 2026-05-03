mod control_flow_layout;
mod graph;
mod model;
mod tree_layout;

use std::collections::{HashMap, HashSet};

use coordinate_systems::World;
use eframe::egui::{Color32, ComboBox, Response, Stroke, Ui, Widget, pos2};
use linear_algebra::{Point2, point, vector};
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

use self::{
    control_flow_layout::build_control_flow_layout,
    graph::{CircleNode, Connection, resolve_circle_collisions},
    model::{
        LayoutViewMode, all_subtree_ids, anchor_position_for_removed_node, collect_statuses_by_id,
        initially_collapsed_subtree_ids, is_descendant_of, parent_position_for_node, status_color,
    },
    tree_layout::build_tree_layout,
};

const LAYOUT_ANIMATION_FACTOR: f32 = 0.2;
const LAYOUT_ANIMATION_EPSILON: f32 = 0.02;
const EXIT_FADE_STEP: f32 = 0.12;
const ENTER_FADE_STEP: f32 = 0.12;
const VIEW_SWITCH_FOCUS_SCALE: f32 = 0.42;
const VIEW_SWITCH_ROOT_Y_OFFSET: f32 = 0.1;

pub struct BehaviorTreePanel {
    tree_layout_buffer: BufferHandle<Option<NodeTrace>>,
    trace_buffer: BufferHandle<Option<NodeTrace>>,
    tree_layout: Option<NodeTrace>,
    collapsed_subtrees: HashSet<String>,
    initial_collapse_applied: bool,
    invert_tree_vertical: bool,
    view_mode: LayoutViewMode,
    pending_view_switch_focus: bool,
    circle_nodes: Vec<CircleNode>,
    exiting_nodes: Vec<CircleNode>,
    connections: Vec<Connection>,
    zoom_and_pan: ZoomAndPanTransform,
}

impl BehaviorTreePanel {
    fn apply_vertical_flip_around_root(&mut self) {
        let pivot_y = self
            .circle_nodes
            .iter()
            .find(|node| node.id == "root")
            .map(|node| node.position.y());

        let Some(pivot_y) = pivot_y else {
            return;
        };

        for node in &mut self.circle_nodes {
            if node.id == "root" {
                continue;
            }

            let y = node.position.y();
            node.position = point![node.position.x(), pivot_y - (y - pivot_y)];
        }
    }

    fn rebuild_layout(&mut self, opening_subtree_origin: Option<&str>) {
        let old_nodes: HashMap<String, CircleNode> = self
            .circle_nodes
            .drain(..)
            .map(|node| (node.id.clone(), node))
            .collect();
        let old_positions: HashMap<String, Point2<World>> = old_nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.position))
            .collect();
        self.connections.clear();

        if let Some(tree_layout) = &self.tree_layout {
            if self.view_mode == LayoutViewMode::Tree {
                let mut next_x = 0.0;
                let mut path = Vec::new();
                build_tree_layout(
                    &mut self.circle_nodes,
                    &mut self.connections,
                    tree_layout,
                    0,
                    &mut next_x,
                    &mut path,
                    &self.collapsed_subtrees,
                );
            } else {
                build_control_flow_layout(
                    &mut self.circle_nodes,
                    &mut self.connections,
                    tree_layout,
                    &self.collapsed_subtrees,
                );
            }

            if self.invert_tree_vertical {
                self.apply_vertical_flip_around_root();
            }

            let visible_target_positions: HashMap<String, Point2<World>> = self
                .circle_nodes
                .iter()
                .map(|node| (node.id.clone(), node.position))
                .collect();

            for (node_id, old_node) in &old_nodes {
                if visible_target_positions.contains_key(node_id) {
                    continue;
                }

                let mut exiting_node = old_node.clone();
                exiting_node.is_dragging = false;
                exiting_node.opacity = 1.0;
                exiting_node.target_position =
                    anchor_position_for_removed_node(node_id, &visible_target_positions)
                        .unwrap_or(exiting_node.position);
                self.exiting_nodes.push(exiting_node);
            }

            for node in &mut self.circle_nodes {
                node.target_position = node.position;

                if let Some(old_node) = old_nodes.get(&node.id) {
                    node.position = old_node.position;
                    node.opacity = old_node.opacity;
                } else {
                    let opening_origin_position = opening_subtree_origin
                        .filter(|origin_id| is_descendant_of(&node.id, origin_id))
                        .and_then(|origin_id| old_positions.get(origin_id).copied());

                    node.position = opening_origin_position
                        .or_else(|| parent_position_for_node(&node.id, &old_positions))
                        .or_else(|| visible_target_positions.get("root").copied())
                        .unwrap_or(node.position);
                    node.opacity = 0.0;
                }
            }
        }
    }

    fn animate_layout(&mut self) -> bool {
        let mut any_animating = false;

        for node in &mut self.circle_nodes {
            if node.is_dragging {
                continue;
            }

            let delta = node.target_position - node.position;
            if delta.inner.norm_squared() > LAYOUT_ANIMATION_EPSILON * LAYOUT_ANIMATION_EPSILON {
                node.position += delta * LAYOUT_ANIMATION_FACTOR;
                any_animating = true;
            } else {
                node.position = node.target_position;
            }

            node.opacity = (node.opacity + ENTER_FADE_STEP).min(1.0);
        }

        self.exiting_nodes.retain_mut(|node| {
            let delta = node.target_position - node.position;
            if delta.inner.norm_squared() > LAYOUT_ANIMATION_EPSILON * LAYOUT_ANIMATION_EPSILON {
                node.position += delta * LAYOUT_ANIMATION_FACTOR;
                any_animating = true;
            } else {
                node.position = node.target_position;
            }

            node.opacity = (node.opacity - EXIT_FADE_STEP).max(0.0);
            if node.opacity > 0.0 {
                any_animating = true;
                true
            } else {
                false
            }
        });
        any_animating
    }

    fn update_layout_if_needed(&mut self) -> bool {
        let tree_layout = self
            .tree_layout_buffer
            .get_last_value()
            .ok()
            .flatten()
            .flatten();

        let Some(tree_layout) = tree_layout else {
            return false;
        };

        let first_layout_load = self.tree_layout.is_none();

        if !self.initial_collapse_applied {
            self.collapsed_subtrees = initially_collapsed_subtree_ids(&tree_layout);
            self.initial_collapse_applied = true;
        }

        if self.tree_layout.as_ref().map(|layout| &layout.name) != Some(&tree_layout.name)
            || self.circle_nodes.is_empty()
        {
            self.tree_layout = Some(tree_layout);
            self.rebuild_layout(None);

            if first_layout_load {
                self.pending_view_switch_focus = true;
            }
            return true;
        }

        false
    }

    fn update_trace_strokes(&mut self) {
        if !self.trace_buffer.has_changed() {
            return;
        }
        self.trace_buffer.mark_as_seen();

        let trace = match self.trace_buffer.get_last_value().ok().flatten().flatten() {
            Some(trace) => trace,
            None => return,
        };

        for node in &mut self.circle_nodes {
            node.stroke = Stroke::new(0.1, Color32::LIGHT_GRAY);
        }

        let mut statuses = HashMap::new();
        let mut path = Vec::new();
        collect_statuses_by_id(&trace, &mut path, &mut statuses);

        for node in &mut self.circle_nodes {
            if let Some(status) = statuses.get(&node.id) {
                node.stroke = Stroke::new(0.1, status_color(status));
            }
        }
    }
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        Self {
            tree_layout_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.tree_layout"),
            trace_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.trace"),
            tree_layout: None,
            collapsed_subtrees: HashSet::new(),
            initial_collapse_applied: false,
            invert_tree_vertical: false,
            view_mode: LayoutViewMode::Tree,
            pending_view_switch_focus: false,
            circle_nodes: Vec::new(),
            exiting_nodes: Vec::new(),
            connections: Vec::new(),
            zoom_and_pan: ZoomAndPanTransform::default(),
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        self.update_layout_if_needed();

        ui.horizontal(|ui| {
            let previous_view_mode = self.view_mode;
            ComboBox::from_label("View")
                .selected_text(self.view_mode.label())
                .show_ui(ui, |ui| {
                    for mode in [LayoutViewMode::Tree, LayoutViewMode::SequenceChains] {
                        ui.selectable_value(&mut self.view_mode, mode, mode.label());
                    }
                });

            if ui.checkbox(&mut self.invert_tree_vertical, "; )").changed() {
                self.rebuild_layout(None);
            }

            if self.view_mode != previous_view_mode {
                self.pending_view_switch_focus = true;
                self.rebuild_layout(None);
            }

            if ui.button("Reset").clicked() {
                self.exiting_nodes.clear();

                if let Some(tree_layout) = &self.tree_layout {
                    self.collapsed_subtrees = initially_collapsed_subtree_ids(tree_layout);
                    self.initial_collapse_applied = true;
                    self.rebuild_layout(None);
                }
            }

            if ui.button("Collapse All").clicked()
                && let Some(tree_layout) = &self.tree_layout
            {
                self.collapsed_subtrees = all_subtree_ids(tree_layout);
                self.rebuild_layout(None);
            }

            if ui.button("Expand All").clicked() {
                self.collapsed_subtrees.clear();
                self.rebuild_layout(None);
            }
        });

        self.update_trace_strokes();

        let (response, mut painter) = TwixPainter::<World>::allocate(
            ui,
            vector![25.0, 25.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );

        let desired_screen = pos2(
            response.rect.center().x,
            response.rect.center().y - response.rect.height() * VIEW_SWITCH_ROOT_Y_OFFSET,
        );

        let root_pixel = self
            .circle_nodes
            .iter()
            .find(|node| node.id == "root")
            .map(|root_node| painter.transform_world_to_pixel(root_node.target_position));

        if self.pending_view_switch_focus {
            if let Some(pixel) = root_pixel {
                self.zoom_and_pan
                    .center_on(pixel, desired_screen, VIEW_SWITCH_FOCUS_SCALE);
            }
            self.pending_view_switch_focus = false;
        }

        let reset_transform = root_pixel.map(|pixel| {
            self.zoom_and_pan
                .centering_transform(pixel, desired_screen, VIEW_SWITCH_FOCUS_SCALE)
        });

        let mut drag_claimed = false;
        self.zoom_and_pan.apply_transform(&mut painter);

        for circle_node in &mut self.circle_nodes {
            circle_node.update(&response, &painter, &mut drag_claimed);
        }

        if response.clicked()
            && let Some(pointer_position) = response.interact_pointer_pos()
            && let Some(clicked_subtree_id) = self
                .circle_nodes
                .iter()
                .rev()
                .find(|node| {
                    node.is_subtree
                        && node.contains(painter.transform_pixel_to_world(pointer_position))
                })
                .map(|node| node.id.clone())
        {
            if self.collapsed_subtrees.contains(&clicked_subtree_id) {
                self.collapsed_subtrees.remove(&clicked_subtree_id);
                self.rebuild_layout(Some(&clicked_subtree_id));
            } else {
                self.collapsed_subtrees.insert(clicked_subtree_id);
                self.rebuild_layout(None);
            }
            drag_claimed = true;
        }

        let is_animating = self.animate_layout();
        if is_animating {
            ui.ctx().request_repaint();
        } else {
            resolve_circle_collisions(&mut self.circle_nodes);
        }

        if !drag_claimed {
            self.zoom_and_pan
                .process_input(ui, &mut painter, &response, reset_transform);
        }

        for connection in &self.connections {
            connection.draw(&mut painter, &self.circle_nodes);
        }

        for circle_node in &self.exiting_nodes {
            circle_node.draw(&mut painter);
        }

        for circle_node in &self.circle_nodes {
            circle_node.draw(&mut painter);
        }

        response
    }
}
