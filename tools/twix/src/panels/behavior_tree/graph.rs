use coordinate_systems::World;
use eframe::egui::{Align2, Color32, FontId, Response, Stroke};
use linear_algebra::{Point2, distance_squared};

use crate::twix_painter::TwixPainter;

const COLLISION_LIMIT: usize = 50;

#[derive(Debug, Clone)]
pub struct CircleNode {
    pub id: String,
    pub is_visible: bool,
    pub opacity: f32,
    pub is_dragging: bool,
    pub is_subtree: bool,
    pub name: String,
    pub position: Point2<World>,
    pub target_position: Point2<World>,
    pub radius: f32,
    pub stroke: Stroke,
}

impl CircleNode {
    pub fn new(
        id: String,
        name: String,
        position: Point2<World>,
        radius: f32,
        stroke: Stroke,
        is_subtree: bool,
        is_visible: bool,
    ) -> Self {
        Self {
            id,
            is_visible,
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
        if !self.is_visible {
            return;
        }

        painter.floating_text(
            self.position,
            Align2::CENTER_CENTER,
            self.name.to_string(),
            FontId::proportional(0.35 * painter.scaling()),
            Color32::WHITE.gamma_multiply(self.opacity),
        );
        painter.circle(
            self.position,
            self.radius,
            Color32::TRANSPARENT,
            Stroke::new(
                self.stroke.width,
                self.stroke.color.gamma_multiply(self.opacity),
            ),
        );

        if self.is_subtree {
            painter.circle(
                self.position,
                self.radius + 0.2,
                Color32::TRANSPARENT,
                Stroke::new(
                    0.1,
                    Color32::from_rgb(255, 165, 0).gamma_multiply(self.opacity),
                ),
            );
        }
    }

    pub fn update(
        &mut self,
        response: &Response,
        painter: &TwixPainter<World>,
        drag_claimed: &mut bool,
    ) {
        if !self.is_visible {
            return;
        }

        if response.drag_started()
            && !*drag_claimed
            && let Some(pointer_position) = response.interact_pointer_pos()
        {
            let world_position = painter.transform_pixel_to_world(pointer_position);
            let distance_squared = distance_squared(world_position, self.position);
            let radius_squared = self.radius * self.radius;

            if distance_squared <= radius_squared {
                self.is_dragging = true;
                *drag_claimed = true;
            }
        }

        if response.dragged()
            && self.is_dragging
            && let Some(pointer_position) = response.interact_pointer_pos()
        {
            self.position = painter.transform_pixel_to_world(pointer_position);
            self.target_position = self.position;
            *drag_claimed = true;
        }

        if response.drag_stopped() {
            self.is_dragging = false;
        }
    }

    fn collision_radius(&self) -> f32 {
        if self.is_subtree {
            self.radius + 0.2
        } else {
            self.radius
        }
    }

    fn collision_stroke_width(&self) -> f32 {
        if self.is_subtree {
            0.1
        } else {
            self.stroke.width
        }
    }

    pub fn contains(&self, point: Point2<World>) -> bool {
        if !self.is_visible {
            return false;
        }

        distance_squared(point, self.position) <= self.radius * self.radius
    }
}

pub fn resolve_circle_collisions(nodes: &mut [CircleNode]) {
    let mut iterations = 0;
    loop {
        let mut did_resolve_any = false;

        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if !nodes[i].is_visible || !nodes[j].is_visible {
                    continue;
                }

                let to_b = nodes[j].position - nodes[i].position;
                let distance_squared = to_b.norm_squared();
                let radius = nodes[i].collision_radius() + nodes[j].collision_radius();
                let minimal_distance =
                    radius + nodes[i].collision_stroke_width() + nodes[j].collision_stroke_width();
                let minimal_distance_squared = minimal_distance * minimal_distance;

                if distance_squared < minimal_distance_squared && distance_squared > f32::EPSILON {
                    let distance = distance_squared.sqrt();
                    let overlap = minimal_distance - distance;
                    let direction = to_b / distance;

                    let a_dragging = nodes[i].is_dragging;
                    let b_dragging = nodes[j].is_dragging;

                    if a_dragging && !b_dragging {
                        nodes[j].position += direction * overlap;
                        nodes[j].target_position = nodes[j].position;
                    } else if !a_dragging && b_dragging {
                        nodes[i].position -= direction * overlap;
                        nodes[i].target_position = nodes[i].position;
                    } else if !a_dragging && !b_dragging {
                        let half_overlap = overlap * 0.5;
                        nodes[i].position -= direction * half_overlap;
                        nodes[i].target_position = nodes[i].position;
                        nodes[j].position += direction * half_overlap;
                        nodes[j].target_position = nodes[j].position;
                    }

                    did_resolve_any = true;
                }
            }
        }

        iterations += 1;

        if !did_resolve_any || iterations > COLLISION_LIMIT {
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
