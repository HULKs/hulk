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
}

impl NegativeEvidence {
    pub fn pause(&mut self) {
        self.last_clear_frame = None;
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

/// Require the complete predicted ball footprint to be observable and a known
/// obstacle snapshot in the same Ground frame. Obstacle radii approximate opaque
/// vertical columns; this type carries neither height nor source confidence.
/// This tests the predicted footprint, not the full positional covariance region.
pub fn clearly_visible(
    ball: &BallPosition<Ground>,
    camera: &CameraMatrix,
    ball_radius: f32,
    obstacles: Option<&[Obstacle]>,
) -> bool {
    let Some(obstacles) = obstacles else {
        return false;
    };
    let Ok(center) = camera.ground_with_z_to_pixel(ball.position, ball_radius) else {
        return false;
    };
    let Ok(radius) = camera.get_pixel_radius(ball_radius, center) else {
        return false;
    };
    if !center.x().is_finite()
        || !center.y().is_finite()
        || !radius.is_finite()
        || radius < MINIMUM_BALL_RADIUS_PIXELS
    {
        return false;
    }
    let margin = radius + IMAGE_MARGIN_PIXELS;
    let min_x = center.x() - margin;
    let max_x = center.x() + margin;
    let min_y = center.y() - margin;
    let max_y = center.y() + margin;
    if !(min_x >= 0.0
        && min_y >= 0.0
        && max_x < camera.image_size.x()
        && max_y < camera.image_size.y())
    {
        return false;
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
        return false;
    }
    !obstacles.iter().any(|obstacle| {
        if obstacle.kind != ObstacleKind::Robot {
            return false;
        }
        let radius = obstacle
            .radius_at_foot_height
            .max(obstacle.radius_at_hip_height);
        let position = obstacle.position.inner.coords;
        if !position.iter().all(|value| value.is_finite())
            || !obstacle.radius_at_foot_height.is_finite()
            || !obstacle.radius_at_hip_height.is_finite()
            || obstacle.radius_at_foot_height < 0.0
            || obstacle.radius_at_hip_height < 0.0
        {
            // An invalid opaque obstacle cannot establish that the view is clear.
            return true;
        }
        if length_squared <= f32::EPSILON {
            return (position - camera_position).norm_squared() <= (radius + ball_radius).powi(2);
        }
        let along_ray = (position - camera_position).dot(&ray) / length_squared;
        if !(0.0..1.0).contains(&along_ray) {
            return false;
        }
        let perpendicular = position - (camera_position + along_ray * ray);
        perpendicular.norm_squared() <= (radius + ball_radius).powi(2)
    })
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
}
