use std::{f32::consts::LN_2, time::Duration};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use types::{
    ball_position::HypotheticalBallPosition, field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters,
};

use crate::{Heatmap, heatmap_tile_center};

// Allow half a search cell for projection and localization error at the field edge.
pub fn ball_position_near_field(
    field_dimensions: FieldDimensions,
    ball_position: Point2<Field>,
) -> bool {
    let margin = crate::CELL_SIZE / 2.0;
    ball_position.x().abs() <= field_dimensions.length / 2.0 + margin
        && ball_position.y().abs() <= field_dimensions.width / 2.0 + margin
}

impl Heatmap {
    pub fn set_known_ball_position(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: Point2<Field>,
    ) {
        if !ball_position_near_field(field_dimensions, ball_position) {
            return;
        }
        let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position);
        let (length, width) = self.map.dim();
        for x in
            heatmap_point.0.saturating_sub(1)..=heatmap_point.0.saturating_add(1).min(length - 1)
        {
            for y in
                heatmap_point.1.saturating_sub(1)..=heatmap_point.1.saturating_add(1).min(width - 1)
            {
                if (x, y) != heatmap_point {
                    self.map[(x, y)] = self.map[(x, y)].max(0.5);
                }
            }
        }
        self.map[heatmap_point] = 1.0;
    }

    pub fn update_with_hypothetical_ball_positions<'a>(
        &mut self,
        field_dimensions: FieldDimensions,
        hypothetical_ball_positions: impl IntoIterator<Item = &'a HypotheticalBallPosition<Ground>>,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: &SearchSuggestorParameters,
    ) {
        for ball_hypothesis in hypothetical_ball_positions {
            if !ball_hypothesis.validity.is_finite() {
                continue;
            }
            let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
            if !ball_position_near_field(field_dimensions, ball_hypothesis_position) {
                continue;
            }
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_hypothesis_position);
            self.map[heatmap_point] = (self.map[heatmap_point]
                + ball_hypothesis.validity * parameters.own_ball_weight)
                / 2.0;
        }
    }

    pub fn update_with_team_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: Point2<Field>,
        parameters: &SearchSuggestorParameters,
    ) {
        if !ball_position_near_field(field_dimensions, ball_position) {
            return;
        }
        let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position);
        self.map[heatmap_point] = parameters.team_ball_weight;
    }

    pub fn increase_around_last_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        last_ball_position: Point2<Field>,
        elapsed: Duration,
        observation_age: Duration,
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
        let fade_time = parameters.last_ball_priority_fade_time.as_secs_f32();
        let spatial_weight = if fade_time > 0.0 {
            (1.0 - observation_age.as_secs_f32() / fade_time).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field = heatmap_tile_center(field_dimensions, (x, y));
            let distance_to_last_ball = (tile_center_in_field - last_ball_position).norm();
            let gaussian = (-LN_2 * (distance_to_last_ball / half_distance).powi(2)).exp();
            let multiplier = parameters.last_ball_priority_minimum
                + (parameters.last_ball_priority_maximum - parameters.last_ball_priority_minimum)
                    * gaussian
                    * spatial_weight;
            *value += multiplier * rise_fraction;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::point;

    #[test]
    fn last_ball_bias_fades_to_uniform_growth_without_erasing_history() {
        let field = FieldDimensions::SPL_2025;
        let parameters = SearchSuggestorParameters {
            last_ball_priority_minimum: 0.1,
            last_ball_priority_maximum: 1.0,
            last_ball_priority_half_distance: 2.0,
            last_ball_priority_rise_time: Duration::from_secs(10),
            last_ball_priority_fade_time: Duration::from_secs(30),
            ..Default::default()
        };
        let mut contrasts = Vec::new();
        for age in [0, 15, 30, 60] {
            let mut heatmap = Heatmap::new(field);
            heatmap.map.fill(0.3);
            heatmap.increase_around_last_ball(
                field,
                heatmap_tile_center(field, (9, 6)),
                Duration::from_secs(1),
                Duration::from_secs(age),
                &parameters,
            );
            contrasts.push(heatmap.map[(9, 6)] - heatmap.map[(0, 0)]);
            assert!(heatmap.map.iter().all(|value| *value >= 0.31 - 1e-6));
            if age >= 30 {
                assert!(heatmap.map.iter().all(|value| (*value - 0.31).abs() < 1e-6));
            }
        }
        assert!(contrasts[0] > 0.08);
        assert!((contrasts[1] - contrasts[0] * 0.5).abs() < 1e-6);
        assert_eq!(contrasts[2], 0.0);
        assert_eq!(contrasts[3], 0.0);
    }

    #[test]
    fn zero_fade_time_disables_spatial_bias() {
        let field = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field);
        heatmap.increase_around_last_ball(
            field,
            point![0.0, 0.0],
            Duration::from_secs(1),
            Duration::ZERO,
            &SearchSuggestorParameters {
                last_ball_priority_minimum: 0.1,
                last_ball_priority_maximum: 1.0,
                last_ball_priority_rise_time: Duration::from_secs(10),
                last_ball_priority_fade_time: Duration::ZERO,
                ..Default::default()
            },
        );
        assert!(heatmap.map.iter().all(|value| (*value - 0.01).abs() < 1e-6));
    }

    #[test]
    fn own_ball_keeps_search_history_and_current_target() {
        let field = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field);
        heatmap.map[(2, 2)] = 0.8;
        heatmap.map[(8, 6)] = 0.7;
        heatmap.last_maximum_heatmap_position = Some((2, 2));

        heatmap.set_known_ball_position(field, point![0.0, 0.0]);

        assert_eq!(heatmap.map[(2, 2)], 0.8);
        assert_eq!(heatmap.map[(8, 6)], 0.7);
        assert_eq!(heatmap.map[(9, 6)], 1.0);
        assert_eq!(heatmap.last_maximum_heatmap_position, Some((2, 2)));
    }

    #[test]
    fn ball_well_outside_field_does_not_create_edge_priority() {
        let field = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field);

        assert!(!ball_position_near_field(field, point![1.7, 5.3]));
        heatmap.set_known_ball_position(field, point![1.7, 5.3]);

        assert!(heatmap.map.iter().all(|value| *value == 0.0));
        assert!(ball_position_near_field(field, point![4.6, 0.0]));
    }
}
