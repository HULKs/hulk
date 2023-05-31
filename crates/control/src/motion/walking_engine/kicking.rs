use std::time::Duration;
use types::{JointOverride, KickStep, LegJoints};

pub fn apply_joint_overrides(
    kick_step: &KickStep,
    swing_leg: &mut LegJoints<f32>,
    t: Duration,
    strength: f32,
) {
    if let Some(overrides) = &kick_step.hip_pitch_overrides {
        swing_leg.hip_pitch += strength * compute_override(overrides, t);
    }
    if let Some(overrides) = &kick_step.ankle_pitch_overrides {
        swing_leg.ankle_pitch += strength * compute_override(overrides, t);
    }
}

fn compute_override(overrides: &[JointOverride], t: Duration) -> f32 {
    let window = overrides.windows(2).find_map(|window| {
        if t >= window[0].timepoint && t < window[1].timepoint {
            Some((window[0], window[1]))
        } else {
            None
        }
    });

    match window {
        Some((start, end)) => {
            let phase_duration = end.timepoint - start.timepoint;
            let t_in_phase = t - start.timepoint;
            let linear_time =
                (t_in_phase.as_secs_f32() / phase_duration.as_secs_f32()).clamp(0.0, 1.0);
            (1.0 - linear_time) * start.value + linear_time * end.value
        }
        None => 0.0,
    }
}
