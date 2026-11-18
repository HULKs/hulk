use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::time::Time;
use search_heatmap::{Heatmap, SearchOccluder, ball_position_in_field};
use std::time::Duration;
use types::{
    ball_position::BallPosition,
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::{Obstacle, ObstacleKind},
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};

pub(crate) fn observe_local_ball(
    ball_position: Option<BallPosition<Ground>>,
    ground_to_field: Option<Isometry2<Ground, Field>>,
    field_dimensions: FieldDimensions,
    last_ball_position: &mut Option<(Point2<Field>, Time)>,
    current_ball_position: &mut Option<Point2<Field>>,
) -> bool {
    match (ball_position, ground_to_field) {
        (Some(ball_position), Some(ground_to_field)) => {
            if !ball_position.position.x().is_finite() || !ball_position.position.y().is_finite() {
                return false;
            }
            let last_seen = ball_position.last_seen;
            let ball_position = ground_to_field * ball_position.position;
            if ball_position_in_field(field_dimensions, ball_position) {
                *last_ball_position = Some((ball_position, last_seen));
                *current_ball_position = Some(ball_position);
                true
            } else {
                *current_ball_position = None;
                false
            }
        }
        (None, _) | (Some(_), None) => {
            *current_ball_position = None;
            false
        }
    }
}

pub(crate) struct PriorityUpdate<'a> {
    pub heatmap: &'a mut Heatmap,
    pub field_dimensions: FieldDimensions,
    pub parameters: &'a SearchSuggestorParameters,
    pub latest_filtered_game_controller_state: &'a Option<FilteredGameControllerState>,
    pub primary_state: Option<&'a PrimaryState>,
    pub last_ball_position: Option<(Point2<Field>, Time)>,
    pub current_ball_position: Option<Point2<Field>>,
    pub ball_was_seen: bool,
    pub is_playing: bool,
    pub entered_playing: bool,
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub obstacles: Option<&'a [Obstacle]>,
    pub head_yaw: f32,
    pub now: Time,
    pub elapsed_since_last_priority_update: Duration,
}

pub(crate) fn update_priorities(input: PriorityUpdate<'_>) {
    let PriorityUpdate {
        heatmap,
        field_dimensions,
        parameters,
        latest_filtered_game_controller_state,
        primary_state,
        last_ball_position,
        current_ball_position,
        ball_was_seen,
        is_playing,
        entered_playing,
        ground_to_field,
        obstacles,
        head_yaw,
        now,
        elapsed_since_last_priority_update,
    } = input;
    let restart_regenerated = latest_filtered_game_controller_state.as_ref().is_some_and(
        |filtered_game_controller_state| {
            heatmap.regenerate_restart_hypotheses(
                filtered_game_controller_state,
                field_dimensions,
                elapsed_since_last_priority_update.as_secs_f32()
                    / parameters.rule_priority_rise_time.as_secs_f32(),
            )
        },
    );
    if !restart_regenerated
        && let (Some(filtered_game_controller_state), Some(primary_state)) = (
            latest_filtered_game_controller_state.as_ref(),
            primary_state,
        )
    {
        heatmap.update_with_rule_ball(
            filtered_game_controller_state,
            field_dimensions,
            *primary_state,
            parameters,
            elapsed_since_last_priority_update,
        );
    }

    if !restart_regenerated
        && is_playing
        && current_ball_position.is_none()
        && let Some((last_ball_position, last_seen)) = last_ball_position
    {
        heatmap.increase_around_last_ball(
            field_dimensions,
            last_ball_position,
            if entered_playing {
                Duration::ZERO
            } else {
                elapsed_since_last_priority_update
            },
            now.duration_since(last_seen),
            parameters,
        );
    }

    if current_ball_position.is_none()
        && !ball_was_seen
        && let Some(ground_to_field) = ground_to_field
    {
        let occluders = search_occluders_from_obstacles(obstacles.unwrap_or(&[]), ground_to_field);
        if head_yaw.is_finite() {
            heatmap.decay_tiles_in_robot_fov_with_occluders(
                field_dimensions,
                ground_to_field,
                head_yaw,
                elapsed_since_last_priority_update,
                parameters,
                &occluders,
            );
        }
    }

    if let Some(current_ball_position) = current_ball_position {
        heatmap.set_known_ball_position(field_dimensions, current_ball_position);
    }

    heatmap.clamp_values();
}

fn search_occluders_from_obstacles(
    obstacles: &[Obstacle],
    ground_to_field: Isometry2<Ground, Field>,
) -> Vec<SearchOccluder> {
    obstacles
        .iter()
        .filter(|obstacle| obstacle.kind != ObstacleKind::Ball)
        .map(|obstacle| SearchOccluder {
            center: ground_to_field * obstacle.position,
            radius: obstacle
                .radius_at_foot_height
                .max(obstacle.radius_at_hip_height),
        })
        .filter(|occluder| {
            occluder.center.x().is_finite()
                && occluder.center.y().is_finite()
                && occluder.radius.is_finite()
                && occluder.radius > 0.0
        })
        .collect()
}
