//! Optional per-second retention learned from consecutive usable detector frames.
use std::time::Duration;

use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::parameters::BallFilterParameters;

use crate::{BallHypothesis, negative_evidence::Visibility};

const MAXIMUM_EXPOSURE_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct Evidence {
    pub time: Time,
    pub visibility: Visibility,
}

pub fn enabled(parameters: &BallFilterParameters) -> bool {
    parameters.hidden_validity_decay_rate.is_some()
        || parameters.visible_missed_validity_decay_rate.is_some()
}

/// The legacy matched-track confidence update remains unchanged. Unmatched
/// tracks use a configured rate only between consecutive, known exposures in
/// the same category; unknown frames, category transitions and gaps add no time.
pub fn factor(
    hypothesis: &mut BallHypothesis,
    time: Time,
    visibility: Visibility,
    matched: bool,
    legacy_factor: f32,
    parameters: &BallFilterParameters,
) -> f32 {
    if matched || !enabled(parameters) {
        hypothesis.validity_decay_evidence = None;
        return legacy_factor;
    }
    let rate = match visibility {
        Visibility::Visible => parameters.visible_missed_validity_decay_rate,
        Visibility::Hidden => parameters.hidden_validity_decay_rate,
        Visibility::Unknown => {
            hypothesis.validity_decay_evidence = None;
            return 1.0;
        }
    };
    if hypothesis
        .validity_decay_evidence
        .as_ref()
        .is_some_and(|previous| previous.time >= time)
    {
        return 1.0;
    }
    let previous = hypothesis
        .validity_decay_evidence
        .replace(Evidence { time, visibility });
    let Some(rate) = rate else {
        return legacy_factor;
    };
    if !rate.is_finite() || rate < 0.0 {
        return 1.0;
    }
    let Some(previous) =
        previous.filter(|previous| previous.visibility == visibility && previous.time < time)
    else {
        return 1.0;
    };
    let interval = time.duration_since(previous.time);
    if interval > MAXIMUM_EXPOSURE_INTERVAL {
        return 1.0;
    }
    (-rate * interval.as_secs_f32()).exp()
}
