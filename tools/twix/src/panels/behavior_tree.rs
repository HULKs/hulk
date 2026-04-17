use std::collections::{HashMap, HashSet};

use coordinate_systems::World;
use eframe::egui::{Align2, Color32, FontId, Response, Stroke, Ui, Widget};
use linear_algebra::{IntoTransform, Point2, distance, point, vector};
use nalgebra::Similarity2;
use types::behavior_tree::{NodeTrace, Status};

use crate::{
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

const COLLIISION_LIMIT: usize = 50;
const X_SPACING: f32 = 5.0;
const NODE_RADIUS: f32 = 2.0;
const Y_SPACING: f32 = 5.0;
const SUBTREE_PREFIX: &str = "subtree_";
const LAYOUT_ANIMATION_FACTOR: f32 = 0.2;
const LAYOUT_ANIMATION_EPSILON: f32 = 0.02;
const EXIT_FADE_STEP: f32 = 0.12;
const ENTER_FADE_STEP: f32 = 0.12;

#[derive(Debug, Clone)]
pub struct CircleNode {
    id: String,
    opacity: f32,
    is_dragging: bool,
    is_subtree: bool,
    name: String,
    position: Point2<World>,
    target_position: Point2<World>,
    radius: f32,
    stroke: Stroke,
}

impl CircleNode {
    pub fn new(
        id: String,
        name: String,
        position: Point2<World>,
        radius: f32,
        stroke: Stroke,
        is_subtree: bool,
    ) -> Self {
        Self {
            id,
            opacity: 1.0,
            is_dragging: false,
            is_subtree,
            name,
            position,
            target_position: position,
            radius,
            stroke,
        }
    }

    pub fn draw(&self, painter: &mut TwixPainter<World>) {
        painter.floating_text(
            self.position,
            Align2::CENTER_CENTER,
            self.name.clone(),
            FontId::proportional(0.35 * painter.scaling()),
            Color32::WHITE.gamma_multiply(self.opacity),
        );
        painter.circle(
            self.position,
            self.radius,
            Color32::TRANSPARENT,
            Stroke::new(self.stroke.width, self.stroke.color.gamma_multiply(self.opacity)),
        );

        if self.is_subtree {
            painter.circle(
                self.position,
                self.radius + 0.2,
                Color32::TRANSPARENT,
                Stroke::new(0.1, Color32::from_rgb(255, 165, 0).gamma_multiply(self.opacity)),
            );
        }
    }

    pub fn update(
        &mut self,
        response: &Response,
        painter: &TwixPainter<World>,
        drag_claimed: &mut bool,
    ) {
        if response.drag_started() && !*drag_claimed {
            if let Some(pointer_position) = response.interact_pointer_pos() {
                let world_position = painter.transform_pixel_to_world(pointer_position);
                let distance = distance(world_position, self.position);

                if distance <= self.radius {
                    self.is_dragging = true;
                    *drag_claimed = true;
                }
            }
        }

        if response.dragged() && self.is_dragging {
            if let Some(pointer_position) = response.interact_pointer_pos() {
                self.position = painter.transform_pixel_to_world(pointer_position);
                self.target_position = self.position;
                *drag_claimed = true;
            }
        }

        if response.drag_stopped() {
            self.is_dragging = false;
        }
    }

    pub fn contains(&self, point: Point2<World>) -> bool {
        distance(point, self.position) <= self.radius
    }
}

pub fn resolve_circle_collisions(nodes: &mut [CircleNode]) {
    let mut iterations = 0;

    loop {
        let mut did_resolve_any = false;

        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                if i == j {
                    continue;
                }
                let distance = distance(nodes[i].position, nodes[j].position)
                    - nodes[i].stroke.width
                    - nodes[j].stroke.width;
                let minimal_distance = nodes[i].radius + nodes[j].radius;

                if distance < minimal_distance && distance > f32::EPSILON {
                    let overlap = minimal_distance - distance;
                    let direction = (nodes[j].position - nodes[i].position).normalize();

                    let a_dragging = nodes[i].is_dragging;
                    let b_dragging = nodes[j].is_dragging;

                    if a_dragging && !b_dragging {
                        nodes[j].position += direction * overlap;
                    } else if !a_dragging && b_dragging {
                        nodes[i].position -= direction * overlap;
                    } else if !a_dragging && !b_dragging {
                        nodes[i].position -= direction * (overlap * 0.5);
                        nodes[j].position += direction * (overlap * 0.5);
                    }

                    did_resolve_any = true;
                }
            }
        }

        iterations += 1;

        if !did_resolve_any || iterations > COLLIISION_LIMIT {
            break;
        }
    }
}

pub struct Connection {
    from: usize,
    to: usize,
    stroke: Stroke,
}

impl Connection {
    pub fn new(from: usize, to: usize, stroke: Stroke) -> Self {
        Self { from, to, stroke }
    }

    pub fn draw(&self, painter: &mut TwixPainter<World>, circle_nodes: &[CircleNode]) {
        if let (Some(from_node), Some(to_node)) =
            (circle_nodes.get(self.from), circle_nodes.get(self.to))
        {
            let position_a = from_node.position;
            let position_b = to_node.position;

            let direction = position_b - position_a;

            let start =
                position_a + direction.normalize() * (from_node.radius + from_node.stroke.width);
            let end = position_b - direction.normalize() * (to_node.radius + to_node.stroke.width);

            painter.line_segment(start, end, self.stroke);
        }
    }
}

pub struct BehaviorTreePanel {
    tree_layout_buffer: BufferHandle<Option<NodeTrace>>,
    trace_buffer: BufferHandle<Option<NodeTrace>>,
    tree_layout: Option<NodeTrace>,
    collapsed_subtrees: HashSet<String>,
    opening_subtree_origin: Option<String>,
    circle_nodes: Vec<CircleNode>,
    exiting_nodes: Vec<CircleNode>,
    connections: Vec<Connection>,
    zoom_and_pan: ZoomAndPanTransform,
}

impl BehaviorTreePanel {
    fn rebuild_layout(&mut self) {
        let old_nodes: HashMap<String, CircleNode> = self
            .circle_nodes
            .iter()
            .cloned()
            .map(|node| (node.id.clone(), node))
            .collect();
        let old_positions: HashMap<String, Point2<World>> = old_nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.position))
            .collect();

        self.circle_nodes.clear();
        self.connections.clear();

        if let Some(tree_layout) = &self.tree_layout {
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

            let visible_node_ids: HashSet<String> =
                self.circle_nodes.iter().map(|node| node.id.clone()).collect();
            let visible_target_positions: HashMap<String, Point2<World>> = self
                .circle_nodes
                .iter()
                .map(|node| (node.id.clone(), node.position))
                .collect();

            for (node_id, old_node) in &old_nodes {
                if visible_node_ids.contains(node_id) {
                    continue;
                }

                let mut exiting_node = old_node.clone();
                exiting_node.is_dragging = false;
                exiting_node.opacity = 1.0;
                exiting_node.target_position = anchor_position_for_removed_node(
                    node_id,
                    &visible_node_ids,
                    &visible_target_positions,
                )
                .unwrap_or(exiting_node.position);
                self.exiting_nodes.push(exiting_node);
            }

            for node in &mut self.circle_nodes {
                let layout_position = node.position;
                node.target_position = layout_position;
                if let Some(old_node) = &old_nodes.get(&node.id) {
                    node.position = old_node.position;
                    node.opacity = old_node.opacity;
                } else {
                    let opening_origin_position = self
                        .opening_subtree_origin
                        .as_deref()
                        .filter(|origin_id| is_descendant_of(&node.id, origin_id))
                        .and_then(|origin_id| old_positions.get(origin_id).copied());

                    node.position = opening_origin_position
                        .or_else(|| parent_position_for_node(&node.id, &old_positions))
                        .or_else(|| visible_target_positions.get("root").copied())
                        .unwrap_or(layout_position);
                    node.opacity = 0.0;
                }
            }
        }

        self.opening_subtree_origin = None;
    }

    fn animate_layout(&mut self) -> bool {
        let mut any_animating = false;

        for node in &mut self.circle_nodes {
            if node.is_dragging {
                continue;
            }

            let delta = node.target_position - node.position;
            if delta.norm() > LAYOUT_ANIMATION_EPSILON {
                node.position += delta * LAYOUT_ANIMATION_FACTOR;
                any_animating = true;
            } else {
                node.position = node.target_position;
            }

            node.opacity = (node.opacity + ENTER_FADE_STEP).min(1.0);
        }

        let mut remaining_exiting_nodes = Vec::with_capacity(self.exiting_nodes.len());
        for mut node in self.exiting_nodes.drain(..) {
            let delta = node.target_position - node.position;
            if delta.norm() > LAYOUT_ANIMATION_EPSILON {
                node.position += delta * LAYOUT_ANIMATION_FACTOR;
                any_animating = true;
            } else {
                node.position = node.target_position;
            }

            node.opacity = (node.opacity - EXIT_FADE_STEP).max(0.0);
            if node.opacity > 0.0 {
                remaining_exiting_nodes.push(node);
                any_animating = true;
            }
        }
        self.exiting_nodes = remaining_exiting_nodes;

        any_animating
    }
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        let circle_nodes = Vec::new();
        let connections = Vec::new();

        Self {
            tree_layout_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.tree_layout"),
            trace_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.trace"),
            tree_layout: None,
            collapsed_subtrees: HashSet::new(),
            opening_subtree_origin: None,
            circle_nodes,
            exiting_nodes: Vec::new(),
            connections,
            zoom_and_pan: ZoomAndPanTransform::default(),
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Some(tree_layout) = self
            .tree_layout_buffer
            .get_last_value()
            .ok()
            .flatten()
            .flatten()
        {
            if self.tree_layout.as_ref().map(|layout| &layout.name) != Some(&tree_layout.name)
                || self.circle_nodes.is_empty()
            {
                self.tree_layout = Some(tree_layout);
                self.rebuild_layout();
            }
        }

        if let Some(trace) = self.trace_buffer.get_last_value().ok().flatten().flatten() {
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

        let (response, mut painter) = TwixPainter::<World>::allocate(
            ui,
            vector![25.0, 25.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );

        let reset_transform = if let Some(first_node) = self.circle_nodes.first() {
            let node_pixel = painter.transform_world_to_pixel(first_node.position);
            let center_pixel = response.rect.center();

            let offset_x = center_pixel.x - node_pixel.x;
            let offset_y = painter.orientation.sign() * (center_pixel.y - node_pixel.y);

            Some(
                Similarity2::new(nalgebra::vector![offset_x, offset_y], 0.0, 1.0)
                    .framed_transform(),
            )
        } else {
            None
        };

        let mut drag_claimed = false;

        self.zoom_and_pan.apply_transform(&mut painter);

        for circle_node in &mut self.circle_nodes {
            circle_node.update(&response, &painter, &mut drag_claimed);
        }

        if response.clicked() {
            if let Some(pointer_position) = response.interact_pointer_pos() {
                let pointer_in_world = painter.transform_pixel_to_world(pointer_position);
                if let Some(clicked_subtree_id) = self
                    .circle_nodes
                    .iter()
                    .rev()
                    .find(|node| node.is_subtree && node.contains(pointer_in_world))
                    .map(|node| node.id.clone())
                {
                    if self.collapsed_subtrees.contains(&clicked_subtree_id) {
                        self.collapsed_subtrees.remove(&clicked_subtree_id);
                        self.opening_subtree_origin = Some(clicked_subtree_id.clone());
                    } else {
                        self.collapsed_subtrees.insert(clicked_subtree_id.clone());
                        self.opening_subtree_origin = None;
                    }
                    self.rebuild_layout();
                    drag_claimed = true;
                }
            }
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

fn build_tree_layout(
    circle_nodes: &mut Vec<CircleNode>,
    connections: &mut Vec<Connection>,
    node_trace: &NodeTrace,
    depth: usize,
    next_x: &mut f32,
    path: &mut Vec<usize>,
    collapsed_subtrees: &HashSet<String>,
) -> usize {
    let node_index = circle_nodes.len();
    let node_id = node_id_from_path(path);
    let subtree_name = node_trace.name.starts_with(SUBTREE_PREFIX);
    let raw_name = node_trace
        .name
        .strip_prefix(SUBTREE_PREFIX)
        .unwrap_or(node_trace.name.as_str());

    let name = match raw_name {
        "Selection" => "?".to_string(),
        "Sequence" => "->".to_string(),
        _ => raw_name.replace(": ", ":\n"),
    };

    circle_nodes.push(CircleNode::new(
        node_id.clone(),
        name,
        point![0.0, depth as f32 * Y_SPACING],
        NODE_RADIUS,
        Stroke::new(0.1, Color32::LIGHT_GRAY),
        subtree_name,
    ));

    if node_trace.children.is_empty()
        || (subtree_name && collapsed_subtrees.contains(&node_id))
    {
        let x = *next_x;
        *next_x += X_SPACING;
        circle_nodes[node_index].position = point![x, depth as f32 * Y_SPACING];
        return node_index;
    }

    let mut child_indices = Vec::with_capacity(node_trace.children.len());
    for (child_index, child) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        let child_idx = build_tree_layout(
            circle_nodes,
            connections,
            child,
            depth + 1,
            next_x,
            path,
            collapsed_subtrees,
        );
        path.pop();

        child_indices.push(child_idx);
        connections.push(Connection::new(
            node_index,
            child_idx,
            Stroke::new(0.1, Color32::LIGHT_GRAY),
        ));
    }

    let sum_x: f32 = child_indices
        .iter()
        .map(|&i| circle_nodes[i].position.x())
        .sum();
    let avg_x = sum_x / child_indices.len() as f32;
    circle_nodes[node_index].position = point![avg_x, depth as f32 * Y_SPACING];

    node_index
}

fn node_id_from_path(path: &[usize]) -> String {
    if path.is_empty() {
        return "root".to_string();
    }

    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn is_descendant_of(node_id: &str, ancestor_id: &str) -> bool {
    node_id != ancestor_id
        && node_id.len() > ancestor_id.len()
        && node_id.starts_with(ancestor_id)
        && node_id.as_bytes()[ancestor_id.len()] == b'.'
}

fn parent_position_for_node(
    node_id: &str,
    old_positions: &HashMap<String, Point2<World>>,
) -> Option<Point2<World>> {
    let parent_id = if let Some((parent, _)) = node_id.rsplit_once('.') {
        parent
    } else if node_id != "root" {
        "root"
    } else {
        return None;
    };

    old_positions.get(parent_id).copied()
}

fn anchor_position_for_removed_node(
    node_id: &str,
    visible_node_ids: &HashSet<String>,
    visible_target_positions: &HashMap<String, Point2<World>>,
) -> Option<Point2<World>> {
    let mut current = node_id.to_string();

    loop {
        if let Some((parent, _)) = current.rsplit_once('.') {
            current = parent.to_string();
        } else if current != "root" {
            current = "root".to_string();
        } else {
            return visible_target_positions.get("root").copied();
        }

        if visible_node_ids.contains(&current) {
            return visible_target_positions.get(&current).copied();
        }
    }
}

fn status_color(status: &Status) -> Color32 {
    match status {
        Status::Success => Color32::CYAN,
        Status::Failure => Color32::RED,
        Status::Idle => Color32::LIGHT_GRAY,
    }
}

fn collect_statuses_by_id(
    node_trace: &NodeTrace,
    path: &mut Vec<usize>,
    statuses: &mut HashMap<String, Status>,
) {
    statuses.insert(node_id_from_path(path), node_trace.status.clone());

    for (child_index, child_trace) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        collect_statuses_by_id(child_trace, path, statuses);
        path.pop();
    }
}
