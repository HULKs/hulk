//! Time spent actually observing a clear, empty candidate location. Sensor gaps,
//! camera fringes and detected robots are not evidence that a hidden ball vanished.
use std::time::Duration;

use coordinate_systems::Ground;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::BallPosition,
    obstacles::{Obstacle, ObstacleKind},
    parameters::BallFilterParameters,
};

// Bridge at most three 25 Hz image intervals. Longer gaps do not assert that
// the candidate remained visible while no detector results were available.
const MAXIMUM_EXPOSURE_INTERVAL: Duration = Duration::from_millis(120);
// Avoid treating partially clipped or effectively subpixel balls as clear misses.
const IMAGE_MARGIN_PIXELS: f32 = 4.0;
const MINIMUM_BALL_RADIUS_PIXELS: f32 = 2.0;

#[derive(Clone, Debug, Default, Serialize, Deserialize, Message)]
pub struct NegativeEvidence {
    pub visible_missed_duration: Duration,
    pub last_clear_frame: Option<Time>,
    #[serde(default)]
    pub near: Option<NearEvidence>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct NearEvidence {
    pub duration: Duration,
    pub last_clear_frame: Time,
}

pub fn near_enabled(parameters: &BallFilterParameters) -> bool {
    !parameters.near_visible_missed_detection_timeout.is_zero()
        && parameters
            .near_visible_missed_detection_distance
            .is_finite()
        && parameters.near_visible_missed_detection_distance > 0.0
}

pub fn enabled(parameters: &BallFilterParameters) -> bool {
    !parameters.visible_missed_detection_timeout.is_zero() || near_enabled(parameters)
}

impl NegativeEvidence {
    pub fn pause(&mut self) {
        self.last_clear_frame = None;
        self.near = None;
    }

    /// Near-range disappearance must be confirmed locally and continuously;
    /// earlier far-away misses or time spent hidden cannot trigger fast expiry.
    pub fn observe_near_miss(&mut self, time: Time, timeout: Duration) -> (bool, Duration) {
        let mut elapsed = Duration::ZERO;
        if let Some(previous) = &mut self.near {
            if time <= previous.last_clear_frame {
                return (false, elapsed);
            }
            let interval = time.duration_since(previous.last_clear_frame);
            if interval <= MAXIMUM_EXPOSURE_INTERVAL {
                elapsed = interval;
                previous.duration = previous.duration.saturating_add(interval);
                previous.last_clear_frame = time;
            } else {
                self.near = None;
            }
        }
        let near = self.near.get_or_insert(NearEvidence {
            duration: Duration::ZERO,
            last_clear_frame: time,
        });
        (!timeout.is_zero() && near.duration >= timeout, elapsed)
    }

    pub fn observe_clear_miss(&mut self, time: Time, timeout: Duration) -> bool {
        if let Some(previous) = self.last_clear_frame {
            if time <= previous {
                return false;
            }
            let interval = time.duration_since(previous);
            if interval <= MAXIMUM_EXPOSURE_INTERVAL {
                self.visible_missed_duration =
                    self.visible_missed_duration.saturating_add(interval);
            }
        }
        self.last_clear_frame = Some(time);
        !timeout.is_zero() && self.visible_missed_duration >= timeout
    }
}

/// Additional learned close-contact decay, measured only between consecutive
/// near clear misses. The independently configured expiry remains a hard bound.
pub fn near_decay_factor(interval: Duration, parameters: &BallFilterParameters) -> f32 {
    let rate = parameters
        .near_visible_missed_validity_decay_rate
        .filter(|rate| rate.is_finite() && *rate >= 0.0)
        .unwrap_or(0.0);
    (-rate * interval.as_secs_f32()).exp()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum Visibility {
    Visible,
    Hidden,
    Unknown,
}

pub fn clearly_visible(
    ball: &BallPosition<Ground>,
    camera: &CameraMatrix,
    ball_radius: f32,
    obstacles: Option<&[Obstacle]>,
) -> bool {
    classify(ball, camera, ball_radius, obstacles) == Visibility::Visible
}

/// Classify only the current Ground state with same-exposure camera geometry
/// and motion-compensated robot obstacles. Fringe/tiny/invalid projections are
/// unknown, rather than confirmed hidden. Robot radii approximate opaque columns.
pub fn classify(
    ball: &BallPosition<Ground>,
    camera: &CameraMatrix,
    ball_radius: f32,
    obstacles: Option<&[Obstacle]>,
) -> Visibility {
    let Some(obstacles) = obstacles else {
        return Visibility::Unknown;
    };
    if !ball.position.x().is_finite() || !ball.position.y().is_finite() {
        return Visibility::Unknown;
    }
    if obstacles
        .iter()
        .filter(|obstacle| obstacle.kind == ObstacleKind::Robot)
        .any(|obstacle| {
            !obstacle
                .position
                .inner
                .coords
                .iter()
                .all(|value| value.is_finite())
                || !obstacle.radius_at_foot_height.is_finite()
                || !obstacle.radius_at_hip_height.is_finite()
                || obstacle.radius_at_foot_height < 0.0
                || obstacle.radius_at_hip_height < 0.0
        })
    {
        return Visibility::Unknown;
    }
    let center = match camera.ground_with_z_to_pixel(ball.position, ball_radius) {
        Ok(center) => center,
        Err(projection::Error::BehindCamera) => return Visibility::Hidden,
        Err(_) => return Visibility::Unknown,
    };
    let Ok(radius) = camera.get_pixel_radius(ball_radius, center) else {
        return Visibility::Unknown;
    };
    if !center.x().is_finite()
        || !center.y().is_finite()
        || !radius.is_finite()
        || radius < MINIMUM_BALL_RADIUS_PIXELS
    {
        return Visibility::Unknown;
    }
    let margin = radius + IMAGE_MARGIN_PIXELS;
    let min_x = center.x() - margin;
    let max_x = center.x() + margin;
    let min_y = center.y() - margin;
    let max_y = center.y() + margin;
    if max_x < 0.0
        || max_y < 0.0
        || min_x >= camera.image_size.x()
        || min_y >= camera.image_size.y()
    {
        return Visibility::Hidden;
    }
    if !(min_x >= 0.0
        && min_y >= 0.0
        && max_x < camera.image_size.x()
        && max_y < camera.image_size.y())
    {
        return Visibility::Unknown;
    }
    let camera_position = camera
        .ground_to_camera
        .inverse()
        .inner
        .translation
        .vector
        .xy();
    let ball_position = ball.position.inner.coords;
    let ray = ball_position - camera_position;
    let length_squared = ray.norm_squared();
    if !length_squared.is_finite() {
        return Visibility::Unknown;
    }
    let occluded = obstacles
        .iter()
        .filter(|obstacle| obstacle.kind == ObstacleKind::Robot)
        .any(|obstacle| {
            let radius = obstacle
                .radius_at_foot_height
                .max(obstacle.radius_at_hip_height);
            let position = obstacle.position.inner.coords;
            if length_squared <= f32::EPSILON {
                return (position - camera_position).norm_squared()
                    <= (radius + ball_radius).powi(2);
            }
            let along_ray = (position - camera_position).dot(&ray) / length_squared;
            if !(0.0..1.0).contains(&along_ray) {
                return false;
            }
            let perpendicular = position - (camera_position + along_ray * ray);
            perpendicular.norm_squared() <= (radius + ball_radius).powi(2)
        });
    if occluded {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(milliseconds: i64) -> Time {
        Time::from_nanos(milliseconds * 1_000_000)
    }

    #[test]
    fn expiry_counts_only_clear_image_intervals_and_pauses_while_hidden() {
        let timeout = Duration::from_millis(200);
        let mut evidence = NegativeEvidence::default();
        for millis in [0, 40, 80] {
            assert!(!evidence.observe_clear_miss(time(millis), timeout));
        }
        evidence.pause();
        assert!(!evidence.observe_clear_miss(time(10_000), timeout));
        assert_eq!(evidence.visible_missed_duration, Duration::from_millis(80));
        for millis in [10_040, 10_080] {
            assert!(!evidence.observe_clear_miss(time(millis), timeout));
        }
        assert!(evidence.observe_clear_miss(time(10_120), timeout));
    }

    #[test]
    fn long_gaps_and_duplicate_or_old_exposures_do_not_add_misses() {
        let mut evidence = NegativeEvidence::default();
        for millis in [0, 40, 40, 20, 10_000] {
            assert!(!evidence.observe_clear_miss(time(millis), Duration::from_millis(80)));
        }
        assert_eq!(evidence.visible_missed_duration, Duration::from_millis(40));
        assert!(evidence.observe_clear_miss(time(10_040), Duration::from_millis(80)));
    }

    #[test]
    fn near_misses_require_contiguous_local_exposure_and_ignore_far_history() {
        let timeout = Duration::from_millis(120);
        let mut evidence = NegativeEvidence {
            visible_missed_duration: Duration::from_secs(5),
            ..Default::default()
        };
        for millis in [0, 40, 80] {
            assert!(!evidence.observe_near_miss(time(millis), timeout).0);
        }
        assert_eq!(
            evidence.observe_near_miss(time(80), timeout),
            (false, Duration::ZERO)
        );
        assert_eq!(
            evidence.observe_near_miss(time(40), timeout),
            (false, Duration::ZERO)
        );
        assert_eq!(
            evidence.observe_near_miss(time(1000), timeout),
            (false, Duration::ZERO)
        );
        assert_eq!(evidence.near.as_ref().unwrap().duration, Duration::ZERO);
        assert!(!evidence.observe_near_miss(time(1040), timeout).0);
        evidence.pause();
        assert_eq!(
            evidence.observe_near_miss(time(1080), timeout),
            (false, Duration::ZERO)
        );
        for millis in [1120, 1160] {
            assert!(!evidence.observe_near_miss(time(millis), timeout).0);
        }
        assert!(evidence.observe_near_miss(time(1200), timeout).0);
    }
}
