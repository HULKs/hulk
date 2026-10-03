//! A soft single-ball preference, earned by sustained actual leader observations.
use std::time::Duration;

use projection::camera_matrix::CameraMatrix;
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    object_detection::{Object, RobocupObjectLabel},
    obstacles::Obstacle,
    parameters::BallFilterParameters,
};

use crate::{
    BallFilter,
    negative_evidence::{self, Visibility},
};

const MINIMUM_LEADER_VALIDITY: f32 = 10.0;
const LEADER_WARMUP: Duration = Duration::from_secs(1);
const MAXIMUM_EXPOSURE_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct Evidence {
    pub first_match: Time,
    pub last_match: Time,
}

pub fn enabled(parameters: &BallFilterParameters) -> bool {
    parameters
        .competing_hypothesis_validity_decay_rate
        .is_some()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Keep same-exposure detections and Ground obstacle geometry explicit"
)]
pub fn apply(
    filter: &mut BallFilter,
    time: Time,
    matched: &[bool],
    camera: Option<&CameraMatrix>,
    obstacles: Option<&[Obstacle]>,
    detections: &[Object<RobocupObjectLabel>],
    ball_radius: f32,
    parameters: &BallFilterParameters,
) {
    let rate = parameters
        .competing_hypothesis_validity_decay_rate
        .filter(|rate| rate.is_finite() && *rate > 0.0);
    let leader = rate
        .and_then(|_| filter.best_hypothesis(parameters.validity_output_threshold))
        .and_then(|leader| {
            filter
                .hypotheses
                .iter()
                .position(|hypothesis| std::ptr::eq(leader, hypothesis))
        })
        .filter(|&index| {
            let hypothesis = &filter.hypotheses[index];
            matched.get(index).copied().unwrap_or(false)
                && hypothesis.validity.is_finite()
                && hypothesis.validity
                    >= MINIMUM_LEADER_VALIDITY.max(parameters.validity_output_threshold)
                && camera.is_some_and(|camera| {
                    negative_evidence::classify_with_detections(
                        &hypothesis.position(),
                        camera,
                        ball_radius,
                        obstacles,
                        detections,
                    ) != Visibility::Unknown
                })
        });
    for (index, hypothesis) in filter.hypotheses.iter_mut().enumerate() {
        if Some(index) != leader {
            hypothesis.leadership_evidence = None;
        }
    }
    let Some(leader) = leader else {
        return;
    };
    let hypothesis = &mut filter.hypotheses[leader];
    if hypothesis
        .leadership_evidence
        .as_ref()
        .is_some_and(|previous| previous.last_match >= time)
    {
        return;
    }
    let previous = hypothesis
        .leadership_evidence
        .take()
        .filter(|previous| time.duration_since(previous.last_match) <= MAXIMUM_EXPOSURE_INTERVAL);
    let interval = previous.as_ref().map_or(Duration::ZERO, |previous| {
        time.duration_since(previous.last_match)
    });
    let first_match = previous
        .as_ref()
        .map_or(time, |previous| previous.first_match);
    hypothesis.leadership_evidence = Some(Evidence {
        first_match,
        last_match: time,
    });
    // Do not back-charge the interval that established the warmup period.
    let mature_interval = time
        .duration_since(first_match)
        .saturating_sub(LEADER_WARMUP)
        .min(interval);
    let factor = (-rate.unwrap_or_default() * mature_interval.as_secs_f32()).exp();
    for (index, hypothesis) in filter.hypotheses.iter_mut().enumerate() {
        // Every real current-frame match remains protected, including a second
        // real ball or a newly reacquired candidate. Raw validity is not a probability.
        if !matched.get(index).copied().unwrap_or(false)
            && camera.is_some_and(|camera| {
                negative_evidence::classify_with_detections(
                    &hypothesis.position(),
                    camera,
                    ball_radius,
                    obstacles,
                    detections,
                ) == Visibility::Visible
            })
        {
            hypothesis.validity *= factor;
        }
    }
}
