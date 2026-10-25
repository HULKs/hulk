use std::{ops::Range, time::Duration};

use coordinate_systems::{Field, Ground};
use geometry::{
    circle::Circle,
    direction::{Direction, Rotate90Degrees},
    line_segment::LineSegment,
};
use linear_algebra::{Isometry2, Orientation2, Pose2, Vector2, vector};
use types::{
    field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters,
    world_state::PlayerState,
};

use crate::{Heatmap, heatmap_tile_center};

pub type SearchOccluder = Circle<Field>;

struct FieldOfViewDecay<'a> {
    near_half_life: f32,
    far_half_life: f32,
    observation_weight: f32,
    range: Range<f32>,
    elapsed: Duration,
    occluders: &'a [SearchOccluder],
    occluded_factor: f32,
}

impl Heatmap {
    fn decay_tiles_in_fov(
        &mut self,
        field_dimensions: FieldDimensions,
        robot_position: Vector2<Field>,
        left_edge: Vector2<Field>,
        right_edge: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        let shape = self.map.dim();
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field =
                heatmap_tile_center(field_dimensions, shape, (x, y)).coords();
            let robot_to_tile = tile_center_in_field - robot_position;
            let is_inside_sight = get_direction(left_edge, robot_to_tile)
                == Some(Direction::Counterclockwise)
                && get_direction(right_edge, robot_to_tile) == Some(Direction::Clockwise);
            let distance_to_tile = robot_to_tile.norm();

            if is_inside_sight && decay.range.contains(&distance_to_tile) {
                let occlusion_factor =
                    if is_occluded(robot_position, tile_center_in_field, decay.occluders) {
                        decay.occluded_factor.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                let u =
                    (distance_to_tile - decay.range.start) / (decay.range.end - decay.range.start);
                let half_life =
                    decay.near_half_life + u * (decay.far_half_life - decay.near_half_life);
                let exposure =
                    decay.elapsed.as_secs_f32() * decay.observation_weight * occlusion_factor;
                *value *= (-std::f32::consts::LN_2 * exposure / half_life).exp();
            }
        });
    }

    fn decay_tiles_from_field_pose_and_heading(
        &mut self,
        field_dimensions: FieldDimensions,
        pose: Pose2<Field>,
        heading_direction: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        if !pose.position().x().is_finite()
            || !pose.position().y().is_finite()
            || !heading_direction.x().is_finite()
            || !heading_direction.y().is_finite()
            || heading_direction.norm() <= f32::EPSILON
        {
            return;
        }

        let robot_position = pose.position().coords();
        let heading_angle = heading_direction.y().atan2(heading_direction.x());
        let fov_angle_offset = 45.0_f32.to_radians();
        let left_angle = heading_angle - fov_angle_offset;
        let right_angle = heading_angle + fov_angle_offset;
        let left_edge: Vector2<Field> = vector![left_angle.cos(), left_angle.sin()];
        let right_edge: Vector2<Field> = vector![right_angle.cos(), right_angle.sin()];

        self.decay_tiles_in_fov(
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
        previous: &PlayerState,
        current: &PlayerState,
        elapsed: Duration,
        parameters: &SearchSuggestorParameters,
    ) {
        if elapsed.is_zero() || parameters.teammate_observation_weight == 0.0 {
            return;
        }
        let steps = (elapsed.as_secs_f64() / parameters.teammate_replay_step.as_secs_f64())
            .ceil()
            .max(1.0) as usize;
        for step in 0..steps {
            let alpha = (step as f32 + 0.5) / steps as f32;
            let (pose, heading_direction) =
                interpolate_teammate_pose_and_heading(previous, current, alpha);
            self.decay_tiles_from_field_pose_and_heading(
                field_dimensions,
                pose,
                heading_direction,
                FieldOfViewDecay {
                    near_half_life: parameters.observation_half_life_near.as_secs_f32(),
                    far_half_life: parameters.observation_half_life_far.as_secs_f32(),
                    observation_weight: parameters.teammate_observation_weight,
                    range: parameters.observation_distance_range.clone(),
                    elapsed: elapsed.div_f64(steps as f64),
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
        head_yaw: f32,
        elapsed: Duration,
        parameters: &SearchSuggestorParameters,
        occluders: &[SearchOccluder],
    ) {
        if !ground_to_field.translation().x().is_finite()
            || !ground_to_field.translation().y().is_finite()
            || !ground_to_field.orientation().angle().is_finite()
            || !head_yaw.is_finite()
        {
            return;
        }
        let view_orientation = ground_to_field.orientation().angle() + head_yaw;
        let heading_direction: Vector2<Field> =
            vector![view_orientation.cos(), view_orientation.sin()];
        self.decay_tiles_from_field_pose_and_heading(
            field_dimensions,
            ground_to_field.as_pose(),
            heading_direction,
            FieldOfViewDecay {
                near_half_life: parameters.observation_half_life_near.as_secs_f32(),
                far_half_life: parameters.observation_half_life_far.as_secs_f32(),
                observation_weight: 1.0,
                range: parameters.observation_distance_range.clone(),
                elapsed,
                occluders,
                occluded_factor: parameters.occluded_observation_weight,
            },
        );
    }
}

fn interpolate_teammate_pose_and_heading(
    previous: &PlayerState,
    current: &PlayerState,
    alpha: f32,
) -> (Pose2<Field>, Vector2<Field>) {
    let alpha = alpha.clamp(0.0, 1.0);
    let interpolated_position = previous
        .pose
        .position()
        .lerp(current.pose.position(), alpha);
    let previous_heading =
        Orientation2::new(previous.pose.orientation().angle() + previous.head_yaw);
    let current_heading = Orientation2::new(current.pose.orientation().angle() + current.head_yaw);
    let interpolated_heading = previous_heading.slerp(current_heading, alpha);

    (
        Pose2::from_parts(interpolated_position, interpolated_heading),
        interpolated_heading.as_unit_vector(),
    )
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

fn get_direction(base_vector: Vector2<Field>, vector_to_test: Vector2<Field>) -> Option<Direction> {
    let clockwise_normal_vector = base_vector.rotate_90_degrees(Direction::Clockwise);
    let directed_cathetus = clockwise_normal_vector.dot(&vector_to_test);

    match directed_cathetus {
        0.0 => Some(Direction::Collinear),
        f if f > 0.0 => Some(Direction::Clockwise),
        f if f < 0.0 => Some(Direction::Counterclockwise),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_decay_uses_head_yaw() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field_dimensions, 18);
        heatmap.map.fill(1.0);
        let parameters = SearchSuggestorParameters {
            observation_half_life_near: Duration::from_secs(1),
            observation_half_life_far: Duration::from_secs(5),
            occluded_observation_weight: 1.0,
            observation_distance_range: 0.0..3.0,
            ..Default::default()
        };
        let ground_to_field = Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], 0.0);

        heatmap.decay_tiles_in_robot_fov_with_occluders(
            field_dimensions,
            ground_to_field,
            std::f32::consts::FRAC_PI_2,
            Duration::from_millis(20),
            &parameters,
            &[],
        );

        assert!(heatmap.map[(9, 8)] < 1.0);
        assert_eq!(heatmap.map[(12, 6)], 1.0);
    }

    #[test]
    fn invalid_head_yaw_does_not_change_heatmap() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field_dimensions, 18);
        heatmap.map.fill(1.0);
        let parameters = SearchSuggestorParameters {
            observation_half_life_near: Duration::from_secs(1),
            observation_half_life_far: Duration::from_secs(5),
            observation_distance_range: 0.0..3.0,
            ..Default::default()
        };

        heatmap.decay_tiles_in_robot_fov_with_occluders(
            field_dimensions,
            Isometry2::<Ground, Field>::identity(),
            f32::NAN,
            Duration::from_millis(20),
            &parameters,
            &[],
        );

        assert!(heatmap.map.iter().all(|value| *value == 1.0));
    }
    #[test]
    fn half_life_interpolates_in_distance_and_is_independent_of_tick_rate() {
        let field = FieldDimensions::SPL_2025;
        let target = crate::heatmap_tile_center(field, (18, 12), (11, 6));
        let parameters = SearchSuggestorParameters {
            observation_distance_range: 1.0..3.0,
            observation_half_life_near: Duration::from_secs(1),
            observation_half_life_far: Duration::from_secs(5),
            ..Default::default()
        };
        for (distance, half_life) in [(1.0, 1.0), (2.0, 3.0), (2.9, 4.8)] {
            for steps in [1, 10, 50] {
                let mut map = Heatmap::new(field, 18);
                map.map.fill(1.0);
                let pose = Isometry2::from_parts(vector![target.x() - distance, target.y()], 0.0);
                for _ in 0..steps {
                    map.decay_tiles_in_robot_fov_with_occluders(
                        field,
                        pose,
                        0.0,
                        Duration::from_secs_f32(half_life / steps as f32),
                        &parameters,
                        &[],
                    );
                }
                assert!((map.map[(11, 6)] - 0.5).abs() < 2e-6);
            }
        }
    }

    #[test]
    fn team_weight_scales_exposure_and_zero_disables_it() {
        let field = FieldDimensions::SPL_2025;
        let state = PlayerState {
            pose: Pose2::default(),
            head_yaw: 0.0,
            ball_position: None,
            pass: None,
        };
        for (weight, expected) in [(0.0, 1.0), (0.5, 0.5), (1.0, 0.25)] {
            for step in [Duration::from_millis(20), Duration::from_millis(400)] {
                let parameters = SearchSuggestorParameters {
                    teammate_observation_weight: weight,
                    teammate_replay_step: step,
                    observation_half_life_near: Duration::from_secs(1),
                    observation_half_life_far: Duration::from_secs(1),
                    ..Default::default()
                };
                let mut map = Heatmap::new(field, 18);
                map.map.fill(1.0);
                map.decay_tiles_from_teammate_motion(
                    field,
                    &state,
                    &state,
                    Duration::from_secs(2),
                    &parameters,
                );
                assert!((map.map[(11, 6)] - expected).abs() < 2e-6);
            }
        }
    }
}
