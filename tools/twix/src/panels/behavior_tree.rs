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
const X_SPACING: f32 = 4.0;
const Y_SPACING: f32 = 5.0;
const NODE_RADIUS: f32 = 2.0;

#[derive(Debug, Clone)]
pub struct CircleNode {
    is_dragging: bool,
    name: String,
    position: Point2<World>,
    radius: f32,
    stroke: Stroke,
}

impl CircleNode {
    pub fn new(name: String, position: Point2<World>, radius: f32, stroke: Stroke) -> Self {
        Self {
            is_dragging: false,
            name,
            position,
            radius,
            stroke,
        }
    }

    pub fn draw(&self, painter: &mut TwixPainter<World>) {
        painter.floating_text(
            self.position,
            Align2::CENTER_CENTER,
            self.name.clone(),
            FontId::default(),
            Color32::WHITE,
        );
        painter.circle(
            self.position,
            self.radius,
            Color32::TRANSPARENT,
            self.stroke,
        );
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
                *drag_claimed = true;
            }
        }

        if response.drag_stopped() {
            self.is_dragging = false;
        }
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
    circle_nodes: Vec<CircleNode>,
    connections: Vec<Connection>,
    zoom_and_pan: ZoomAndPanTransform,
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
            circle_nodes,
            connections,
            zoom_and_pan: ZoomAndPanTransform::default(),
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.circle_nodes.is_empty() {
            let tree_buffer_value = self
                .tree_layout_buffer
                .get_last_value()
                .ok()
                .flatten()
                .flatten();
            if let Some(tree_layout) = tree_buffer_value {
                let mut next_x = 0.0;
                build_tree_layout(
                    &mut self.circle_nodes,
                    &mut self.connections,
                    &tree_layout,
                    0,
                    &mut next_x,
                );
            }
        } else if let Some(trace) = self.trace_buffer.get_last_value().ok().flatten().flatten() {
            for node in &mut self.circle_nodes {
                node.stroke = Stroke::new(0.1, Color32::LIGHT_GRAY);
            }
            update_status_colors(&trace, 0, &mut self.circle_nodes, &mut self.connections);
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
        resolve_circle_collisions(&mut self.circle_nodes);

        if !drag_claimed {
            self.zoom_and_pan
                .process_input(ui, &mut painter, &response, reset_transform);
        }

        for connection in &self.connections {
            connection.draw(&mut painter, &self.circle_nodes);
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
) -> usize {
    let node_index = circle_nodes.len();

    let name = match node_trace.name.as_str() {
        "Selection" => "?".to_string(),
        "Sequence" => "->".to_string(),
        _ => node_trace.name.clone(),
    };

    circle_nodes.push(CircleNode::new(
        name,
        point![0.0, depth as f32 * Y_SPACING],
        NODE_RADIUS,
        Stroke::new(0.1, Color32::LIGHT_GRAY),
    ));

    if node_trace.children.is_empty() {
        let x = *next_x;
        *next_x += X_SPACING;
        circle_nodes[node_index].position = point![x, depth as f32 * Y_SPACING];
        return node_index;
    }

    let mut child_indices = Vec::with_capacity(node_trace.children.len());
    for child in &node_trace.children {
        let child_idx = build_tree_layout(circle_nodes, connections, child, depth + 1, next_x);
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

fn update_status_colors(
    node_trace: &NodeTrace,
    node_index: usize,
    circle_nodes: &mut Vec<CircleNode>,
    connections: &mut Vec<Connection>,
) {
    let color = match node_trace.status {
        Status::Success => Color32::GREEN,
        Status::Failure => Color32::RED,
        Status::Running => Color32::YELLOW,
        Status::Idle => Color32::LIGHT_GRAY,
    };

    circle_nodes[node_index].stroke = Stroke::new(0.1, color);

    let layout_children: Vec<usize> = connections
        .iter()
        .filter(|c| c.from == node_index)
        .map(|c| c.to)
        .collect();

    for (child_trace, &child_index) in node_trace.children.iter().zip(layout_children.iter()) {
        update_status_colors(child_trace, child_index, circle_nodes, connections);
    }
}
