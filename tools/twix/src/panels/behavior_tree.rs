use coordinate_systems::World;
use eframe::egui::{self, Color32, Response, Stroke, Ui, Widget, accesskit::Point, pos2};
use geometry::circle;
use linear_algebra::{Point2, distance, point, vector};
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

pub struct BehaviorTreePanel {
    trace_buffer: BufferHandle<Option<NodeTrace>>,
    tree_layout_buffer: BufferHandle<Option<NodeTrace>>,
    circle_nodes: Vec<CircleNode>,
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        let mut circle_nodes = Vec::new();
        circle_nodes.push(CircleNode::new(
            Color32::RED,
            "Test".to_string(),
            point![0.0, 0.0],
            2.0,
        ));
        circle_nodes.push(CircleNode::new(
            Color32::GREEN,
            "Test".to_string(),
            point![1.0, 0.0],
            2.0,
        ));
        circle_nodes.push(CircleNode::new(
            Color32::BLUE,
            "Test".to_string(),
            point![2.0, 0.0],
            2.0,
        ));

        Self {
            trace_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.trace"),
            tree_layout_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.tree_layout"),
            circle_nodes,
        }
    }
}

struct Node {
    id: usize,
    pos: Point,
    name: String,
}

struct Connection {
    from: usize,
    to: usize,
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
            vector![10.0, 10.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );

        let mut drag_claimed = false;

        for circle_node in &mut self.circle_nodes {
            circle_node.update(&response, &painter, &mut drag_claimed);
        }

        for circle_node in &self.circle_nodes {
            circle_node.draw(&mut painter);
        }

        response
    }
}

pub struct CircleNode {
    color: Color32,
    name: String,
    position: Point2<World>,
    radius: f32,
    is_dragging: bool,
}

impl CircleNode {
    pub fn new(color: Color32, name: String, position: Point2<World>, radius: f32) -> Self {
        Self {
            color,
            name,
            position,
            radius,
            is_dragging: false,
        }
    }

    pub fn draw(&self, painter: &mut TwixPainter<World>) {
        painter.circle(self.position, self.radius, self.color, Stroke::NONE);
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
