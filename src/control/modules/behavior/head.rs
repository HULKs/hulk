use nalgebra::{point, Point2, UnitComplex, Vector2};
use ordered_float::NotNan;
use types::{FieldDimensions, HeadMotion, WorldState};

use crate::framework::configuration;

#[derive(Debug)]
pub struct LookAction<'cycle> {
    world_state: &'cycle WorldState,
    field_marks: Vec<Point2<f32>>,
    parameters: &'cycle configuration::LookAction,
}

impl<'cycle> LookAction<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        parameters: &'cycle configuration::LookAction,
    ) -> Self {
        let field_marks = generate_field_marks(field_dimensions);
        Self {
            world_state,
            field_marks,
            parameters,
        }
    }

    pub fn execute(&self) -> HeadMotion {
        match self.world_state.ball {
            Some(ball) => HeadMotion::LookAt {
                target: ball.position,
            },
            None => self.look_for_field_marks(),
        }
    }

    fn is_position_visible(&self, position: Point2<f32>) -> bool {
        UnitComplex::rotation_between(&Vector2::x(), &position.coords)
            .angle()
            .abs()
            < self.parameters.angle_threshold
            && position.coords.norm() < self.parameters.distance_threshold
    }

    fn closest_field_mark_visible(&self) -> Option<Point2<f32>> {
        let robot_to_field = self.world_state.robot.robot_to_field?;
        self.field_marks
            .iter()
            .map(|position| robot_to_field.inverse() * position)
            .filter(|position| self.is_position_visible(*position))
            .min_by_key(|position| NotNan::new(position.coords.norm()).unwrap())
    }

    pub fn look_for_field_marks(&self) -> HeadMotion {
        let closest_field_mark_visible = self.closest_field_mark_visible();
        match closest_field_mark_visible {
            Some(target) => HeadMotion::LookAt { target },
            None => HeadMotion::LookAround,
        }
    }
}

fn generate_field_marks(field_dimensions: &FieldDimensions) -> Vec<Point2<f32>> {
    let left_center_circle_junction = point![0.0, field_dimensions.center_circle_diameter / 2.0];
    let right_center_circle_junction = point![0.0, -field_dimensions.center_circle_diameter / 2.0];
    let left_center_t_junction = point![0.0, field_dimensions.width / 2.0];
    let right_center_t_junction = point![0.0, -field_dimensions.width / 2.0];
    let left_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    let left_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    vec![
        left_center_circle_junction,
        right_center_circle_junction,
        left_center_t_junction,
        right_center_t_junction,
        left_opponent_penalty_box_corner,
        right_opponent_penalty_box_corner,
        left_own_penalty_box_corner,
        right_own_penalty_box_corner,
    ]
}
