use std::{f32::consts::LN_2, time::Duration, time::SystemTime};

use coordinate_systems::{Field, Ground};
use hsl_network_messages::{HulkMessage, StateMessage};
use linear_algebra::{Isometry2, Point2, Vector2};
use ros_z::time::Time;
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    time_wrapper::TimeWrapper,
};

use crate::{Heatmap, heatmap_tile_center};

impl Heatmap {
    pub fn set_known_ball_position(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: Point2<Field>,
    ) {
        let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position);
        let (length, width) = self.map.dim();
        self.map.fill(0.0);
        for x in
            heatmap_point.0.saturating_sub(1)..=heatmap_point.0.saturating_add(1).min(length - 1)
        {
            for y in
                heatmap_point.1.saturating_sub(1)..=heatmap_point.1.saturating_add(1).min(width - 1)
            {
                if (x, y) != heatmap_point {
                    self.map[(x, y)] = 0.5;
                }
            }
        }
        self.map[heatmap_point] = 1.0;
        self.last_maximum_heatmap_position = Some(heatmap_point);
        self.has_decided_for_heatmap_tile = true;
    }

    pub fn update_with_hypothetical_ball_positions<'a>(
        &mut self,
        field_dimensions: FieldDimensions,
        hypothetical_ball_positions: impl IntoIterator<Item = &'a HypotheticalBallPosition<Ground>>,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: &SearchSuggestorParameters,
    ) {
        for ball_hypothesis in hypothetical_ball_positions {
            let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_hypothesis_position);
            self.map[heatmap_point] = (self.map[heatmap_point]
                + ball_hypothesis.validity * parameters.own_ball_weight)
                / 2.0;
        }
    }

    pub fn update_with_team_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        network_message: TimeWrapper<IncomingMessage>,
        parameters: &SearchSuggestorParameters,
    ) {
        let IncomingMessage::Hsl(message) = network_message.inner else {
            return;
        };
        self.add_team_ball(
            field_dimensions,
            network_message.time.to_wallclock(),
            message,
            parameters.team_ball_weight,
        );
    }

    fn add_team_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        time: SystemTime,
        message: HulkMessage,
        team_ball_weight: f32,
    ) {
        let ball = match message {
            HulkMessage::State(StateMessage { ball_position, .. }) => {
                ball_position.map(|ball| BallPosition {
                    position: ball.position,
                    velocity: Vector2::zeros(),
                    last_seen: Time::from_wallclock(time) - ball.age,
                })
            }
        };
        if let Some(ball_position) = ball {
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position.position);
            self.map[heatmap_point] = team_ball_weight;
        }
    }

    pub fn increase_around_last_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        last_ball_position: Point2<Field>,
        elapsed: Duration,
        parameters: &SearchSuggestorParameters,
    ) {
        let rise_time = parameters.last_ball_priority_rise_time.as_secs_f32();
        let rise_fraction = if rise_time <= f32::EPSILON {
            1.0
        } else {
            elapsed.as_secs_f32() / rise_time
        };
        if rise_fraction <= 0.0 {
            return;
        }

        let half_distance = parameters
            .last_ball_priority_half_distance
            .max(f32::EPSILON);
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field = heatmap_tile_center(field_dimensions, (x, y));
            let distance_to_last_ball = (tile_center_in_field - last_ball_position).norm();
            let gaussian = (-LN_2 * (distance_to_last_ball / half_distance).powi(2)).exp();
            let multiplier = parameters.last_ball_priority_minimum
                + (parameters.last_ball_priority_maximum - parameters.last_ball_priority_minimum)
                    * gaussian;
            *value += multiplier * rise_fraction;
        });
    }
}
