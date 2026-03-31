use coordinate_systems::World;
use eframe::egui::{Align2, Color32, FontId, Response, Stroke, Ui, Widget};
use linear_algebra::{Point2, distance, point, vector};
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

const COLLIISION_LIMIT: usize = 50;
pub struct BehaviorTreePanel {
    trace_buffer: BufferHandle<Option<NodeTrace>>,
    tree_layout_buffer: BufferHandle<Option<NodeTrace>>,
    circle_nodes: Vec<CircleNode>,
    connections: Vec<Connection>,
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        let mut circle_nodes = Vec::new();
        circle_nodes.push(CircleNode::new(
            "Test".to_string(),
            point![12.0, 3.0],
            2.0,
            Stroke::new(0.1, Color32::RED),
        ));
        circle_nodes.push(CircleNode::new(
            "Test2".to_string(),
            point![5.0, 10.0],
            2.0,
            Stroke::new(0.1, Color32::GREEN),
        ));
        circle_nodes.push(CircleNode::new(
            "Test3".to_string(),
            point![20.0, 15.0],
            2.0,
            Stroke::new(0.1, Color32::BLUE),
        ));

        let mut connections = Vec::new();
        connections.push(Connection::new(0, 1, Stroke::new(0.1, Color32::LIGHT_GRAY)));
        connections.push(Connection::new(0, 2, Stroke::new(0.1, Color32::LIGHT_GRAY)));

        Self {
            trace_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.trace"),
            tree_layout_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.tree_layout"),
            circle_nodes,
            connections,
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        // match self.tree_layout_buffer.get_last_value() {
        //     Ok(Some(layout)) => {
        //         ui.label("Behavior Tree Layout:");
        //         ui.label(format!("{layout:#?}"));
        //     }
        //     _ => {
        //         ui.label("No layout data");
        //     }
        // };

        // match self.trace_buffer.get_last_value() {
        //     Ok(Some(trace)) => ui.label(format!("{trace:#?}")),
        //     _ => ui.label("No data"),
        // };

        let (response, mut painter) = TwixPainter::<World>::allocate(
            ui,
            vector![25.0, 25.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );

        let mut drag_claimed = false;

        for circle_node in &mut self.circle_nodes {
            circle_node.update(&response, &painter, &mut drag_claimed);
        }

        resolve_circle_collisions(&mut self.circle_nodes);

        for connection in &self.connections {
            connection.draw(&mut painter, &self.circle_nodes);
        }

        for circle_node in &self.circle_nodes {
            circle_node.draw(&mut painter);
        }

        response
    }
}

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
