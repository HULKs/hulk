use types::parameters::SearchSuggestorParameters;

pub(crate) fn validate_search_suggestor_parameters(
    parameters: &SearchSuggestorParameters,
) -> std::result::Result<(), String> {
    if !parameters.minimum_search_priority.is_finite()
        || !(0.0..=1.0).contains(&parameters.minimum_search_priority)
    {
        return Err(format!(
            "minimum_search_priority must be finite and between 0 and 1 (got {})",
            parameters.minimum_search_priority
        ));
    }
    if !parameters.fallback_minimum_search_priority.is_finite()
        || !(0.0..=parameters.minimum_search_priority)
            .contains(&parameters.fallback_minimum_search_priority)
    {
        return Err(format!(
            "fallback_minimum_search_priority must be finite and between 0 and minimum_search_priority (got {})",
            parameters.fallback_minimum_search_priority
        ));
    }
    if !(1..=512).contains(&parameters.cells_along_field_length) {
        return Err("cells_along_field_length must be between 1 and 512".into());
    }
    for (name, value) in [
        (
            "rule_priority_rise_time",
            parameters.rule_priority_rise_time,
        ),
        (
            "unseen_priority_rise_time",
            parameters.unseen_priority_rise_time,
        ),
        (
            "last_ball_extra_priority_rise_time",
            parameters.last_ball_extra_priority_rise_time,
        ),
        (
            "observation_half_life_near",
            parameters.observation_half_life_near,
        ),
        (
            "observation_half_life_far",
            parameters.observation_half_life_far,
        ),
        ("teammate_replay_step", parameters.teammate_replay_step),
    ] {
        if value.is_zero() {
            return Err(format!("{name} must be positive"));
        }
    }
    let range = &parameters.observation_distance_range;
    if !range.start.is_finite()
        || !range.end.is_finite()
        || range.start < 0.0
        || range.end <= range.start
    {
        return Err("observation_distance_range requires finite 0 <= start < end".into());
    }
    for (name, value) in [
        (
            "hypothesis_priority_scale",
            parameters.hypothesis_priority_scale,
        ),
        ("team_ball_priority", parameters.team_ball_priority),
        (
            "teammate_observation_weight",
            parameters.teammate_observation_weight,
        ),
        (
            "occluded_observation_weight",
            parameters.occluded_observation_weight,
        ),
        ("border_view_penalty", parameters.border_view_penalty),
        (
            "border_view_penalty_width_fraction",
            parameters.border_view_penalty_width_fraction,
        ),
        ("tile_switch_hysteresis", parameters.tile_switch_hysteresis),
        (
            "turn_preference_priority_margin",
            parameters.turn_preference_priority_margin,
        ),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!("{name} must be finite and between 0 and 1"));
        }
    }
    if !parameters.last_ball_initial_half_distance.is_finite()
        || parameters.last_ball_initial_half_distance <= 0.0
        || !parameters.last_ball_spread_speed.is_finite()
        || parameters.last_ball_spread_speed < 0.0
    {
        return Err(
            "last-ball half distance must be positive and spread speed non-negative, both finite"
                .into(),
        );
    }
    Ok(())
}
