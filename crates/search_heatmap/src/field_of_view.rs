use std::ops::Range;

use coordinate_systems::{Field, Ground};
use geometry::{
    circle::Circle,
    direction::{Direction, Rotate90Degrees},
    line_segment::LineSegment,
};
use hsl_network_messages::StateMessage;
use linear_algebra::{Isometry2, Pose2, Vector2, point, vector};
use nalgebra::clamp;
use types::{field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters};

use crate::Heatmap;

pub type SearchOccluder = Circle<Field>;

struct FieldOfViewDecay<'a> {
    distance_factor: f32,
    range: Range<f32>,
    sampled_tick_count: usize,
    occluders: &'a [SearchOccluder],
    occluded_factor: f32,
}

impl Heatmap {
    fn decay_tiles_in_fov_for_sampled_ticks(
        &mut self,
        field_dimensions: FieldDimensions,
        robot_position: Vector2<Field>,
        left_edge: Vector2<Field>,
        right_edge: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        let sampled_tick_count = decay.sampled_tick_count.max(1).min(i32::MAX as usize) as i32;
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field: Vector2<Field> = vector![
                (x as f32 + 0.5 - field_dimensions.length / 2.0),
                (y as f32 + 0.5 - field_dimensions.width / 2.0)
            ];
            let robot_to_tile = tile_center_in_field - robot_position;
            let is_inside_sight = get_direction(left_edge, robot_to_tile)
                == Direction::Counterclockwise
                && get_direction(right_edge, robot_to_tile) == Direction::Clockwise;
            let distance_to_tile = robot_to_tile.norm();
            let relative_distance_to_tile = clamp(distance_to_tile / decay.range.end, 0.0, 1.0);
            if is_inside_sight && decay.range.contains(&distance_to_tile) {
                let occlusion_factor =
                    if is_occluded(robot_position, tile_center_in_field, decay.occluders) {
                        decay.occluded_factor.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                let per_tick_decay =
                    decay.distance_factor * occlusion_factor * (1.0 - relative_distance_to_tile);
                let effective_decay = 1.0 - (1.0 - per_tick_decay).powi(sampled_tick_count);
                *value *= 1.0 - effective_decay;
            }
        });
    }

    fn decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
        &mut self,
        field_dimensions: FieldDimensions,
        pose: Pose2<Field>,
        heading_direction: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        if heading_direction.norm() <= f32::EPSILON {
            return;
        }

        let robot_position = pose.position().coords();
        let heading_angle = heading_direction.y().atan2(heading_direction.x());
        let fov_angle_offset = 45.0_f32.to_radians();
        let left_angle = heading_angle - fov_angle_offset;
        let right_angle = heading_angle + fov_angle_offset;
        let left_edge: Vector2<Field> = vector![left_angle.cos(), left_angle.sin()];
        let right_edge: Vector2<Field> = vector![right_angle.cos(), right_angle.sin()];

        self.decay_tiles_in_fov_for_sampled_ticks(
            field_dimensions,
            robot_position,
            left_edge,
            right_edge,
            decay,
        );
    }

    pub fn decay_tiles_from_teammate_motion(
        &mut self,
        field_dimensions: FieldDimensions,
        previous: &StateMessage,
        current: &StateMessage,
        tick_count: usize,
        parameters: &SearchSuggestorParameters,
    ) {
        let tick_count = tick_count.max(1);
        let replay_stride = parameters.teammate_replay_stride.max(1);
        for chunk_start in (0..tick_count).step_by(replay_stride) {
            let sampled_tick_count = (tick_count - chunk_start).min(replay_stride);
            let alpha = (chunk_start as f32 + sampled_tick_count as f32 * 0.5) / tick_count as f32;
            let (pose, heading_direction) =
                interpolate_teammate_pose_and_heading(previous, current, alpha);
            self.decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
                field_dimensions,
                pose,
                heading_direction,
                FieldOfViewDecay {
                    distance_factor: teammate_decay_factor(parameters),
                    range: parameters.heatmap_decay_range.clone(),
                    sampled_tick_count,
                    occluders: &[],
                    occluded_factor: 1.0,
                },
            );
        }
    }

    pub fn decay_tiles_in_robot_fov_with_occluders(
        &mut self,
        field_dimensions: FieldDimensions,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: &SearchSuggestorParameters,
        occluders: &[SearchOccluder],
    ) {
        let body_orientation = ground_to_field.orientation().angle();
        let heading_direction: Vector2<Field> =
            vector![body_orientation.cos(), body_orientation.sin()];
        self.decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
            field_dimensions,
            ground_to_field.as_pose(),
            heading_direction,
            FieldOfViewDecay {
                distance_factor: parameters.decay_distance_factor,
                range: parameters.heatmap_decay_range.clone(),
                sampled_tick_count: 1,
                occluders,
                occluded_factor: parameters.occluded_decay_factor,
            },
        );
    }
}

fn teammate_decay_factor(parameters: &SearchSuggestorParameters) -> f32 {
    if parameters.teammate_decay_factor > 0.0 {
        parameters.teammate_decay_factor
    } else {
        parameters.decay_distance_factor * 0.25
    }
}

fn interpolate_teammate_pose_and_heading(
    previous: &StateMessage,
    current: &StateMessage,
    alpha: f32,
) -> (Pose2<Field>, Vector2<Field>) {
    let alpha = alpha.clamp(0.0, 1.0);
    let previous_position = previous.pose.position();
    let current_position = current.pose.position();
    let interpolated_position = point![
        previous_position.x() + (current_position.x() - previous_position.x()) * alpha,
        previous_position.y() + (current_position.y() - previous_position.y()) * alpha
    ];
    let previous_heading = previous.pose.orientation().angle() + previous.head_yaw;
    let current_heading = current.pose.orientation().angle() + current.head_yaw;
    let interpolated_heading =
        previous_heading + normalize_angle(current_heading - previous_heading) * alpha;
    let heading_direction = vector![interpolated_heading.cos(), interpolated_heading.sin()];

    (
        Pose2::new(interpolated_position, interpolated_heading),
        heading_direction,
    )
}

fn normalize_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

fn is_occluded(
    observer: Vector2<Field>,
    tile_center: Vector2<Field>,
    occluders: &[SearchOccluder],
) -> bool {
    let sight_line = LineSegment::new(observer.as_point(), tile_center.as_point());
    if sight_line.length_squared() <= f32::EPSILON {
        return false;
    }

    occluders.iter().any(|occluder| {
        if occluder.radius <= 0.0 {
            return false;
        }
        let projection_along_sight_line = sight_line.projection_factor(occluder.center);
        if !(0.0..1.0).contains(&projection_along_sight_line) {
            return false;
        }
        occluder.intersects_line_segment(&sight_line)
    })
}

fn get_direction(base_vector: Vector2<Field>, vector_to_test: Vector2<Field>) -> Direction {
    let clockwise_normal_vector = base_vector.rotate_90_degrees(Direction::Clockwise);
    let directed_cathetus = clockwise_normal_vector.dot(&vector_to_test);

    match directed_cathetus {
        0.0 => Direction::Collinear,
        f if f > 0.0 => Direction::Clockwise,
        f if f < 0.0 => Direction::Counterclockwise,
        f => panic!("directed cathetus was not a real number: {f}"),
    }
}
