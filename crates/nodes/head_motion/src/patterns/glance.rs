//! Alternating ground targets; gaze geometry and joint trajectories are evaluated by the caller.

use std::time::Duration;

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use coordinate_systems::Ground;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Point2, Rotation2};
use ros_z::time::Time;
use types::{motion_command::ImageRegion, support_foot::Side};

use crate::{joint_control::MotionProgress, parameters::Parameters};

pub struct GlanceTarget {
    pub position: Point2<Ground>,
    pub image_region: ImageRegion,
    pub travel_speed: HeadJoints<f32>,
}

#[derive(Debug)]
pub struct GlanceTimeout {
    pub side: Side,
    pub target: Point2<Ground>,
    pub progress: MotionProgress,
    pub elapsed: Duration,
}

struct PendingGlance {
    target: Point2<Ground>,
    elapsed: Duration,
    maximum_duration: Duration,
}

struct ActiveGlance {
    side: Side,
    elapsed: Duration,
    last_update: Time,
    tracking: bool,
    pending: Option<PendingGlance>,
}

impl ActiveGlance {
    fn new(now: Time) -> Self {
        Self {
            side: Side::Left,
            elapsed: Duration::ZERO,
            last_update: now,
            tracking: false,
            pending: None,
        }
    }
}

#[derive(Default)]
pub struct GlanceState {
    active: Option<ActiveGlance>,
}

impl GlanceState {
    /// Select the current side's target, preserving distance from the Ground origin.
    /// The coordinator retains target height and solves gaze geometry for this point.
    /// Target updates preserve phase; gaps and clock rollback restart on the left.
    /// After joint control evaluates this target, call `advance` with that output's
    /// progress. Never reuse progress from an older target or from a hold command.
    pub fn update(
        &mut self,
        target: Point2<Ground>,
        parameters: &Parameters,
        now: Time,
    ) -> Result<GlanceTarget> {
        parameters
            .glance
            .validate()
            .map_err(|error| eyre!("glance.{error}"))?;
        let active = self.active.get_or_insert_with(|| ActiveGlance::new(now));
        if now < active.last_update
            || now.duration_since(active.last_update) > parameters.joint_control.reseed_after
        {
            *active = ActiveGlance::new(now);
        }
        let elapsed = if active.tracking {
            now.duration_since(active.last_update)
        } else {
            Duration::ZERO
        };
        // Invalid targets still count as activity. advance(None) pauses phase time.
        active.last_update = now;
        ensure!(
            target.inner.coords.iter().all(|value| value.is_finite()),
            "glance target must be finite"
        );
        let angle = match active.side {
            Side::Left => parameters.glance.angle,
            Side::Right => -parameters.glance.angle,
        };
        let position = Rotation2::<Ground, Ground>::new(angle) * target;
        ensure!(
            position.inner.coords.iter().all(|value| value.is_finite()),
            "glance offset target must be finite"
        );
        active.pending = Some(PendingGlance {
            target: position,
            elapsed,
            maximum_duration: parameters.glance.maximum_phase_duration,
        });
        Ok(GlanceTarget {
            position,
            image_region: ImageRegion::Center,
            travel_speed: parameters.glance.travel_speed,
        })
    }

    /// Consume feedback for the just-selected target, switching for the next request.
    /// Position arrival suffices even while moving; there is no dwell. Pass None on
    /// geometry/planning failure to preserve the side and pause its deadline while
    /// the coordinator holds. Each selected target consumes feedback at most once.
    pub fn advance(&mut self, progress: Option<&MotionProgress>) -> Option<GlanceTimeout> {
        let active = self.active.as_mut()?;
        let pending = active.pending.take();
        let Some(progress) = progress else {
            active.tracking = false;
            return None;
        };
        let pending = pending?;
        active.elapsed += pending.elapsed;
        active.tracking = true;
        let expired = active.elapsed >= pending.maximum_duration;
        let timeout = (expired && !progress.position_reached).then_some(GlanceTimeout {
            side: active.side,
            target: pending.target,
            progress: *progress,
            elapsed: active.elapsed,
        });
        if progress.position_reached || expired {
            active.side = active.side.opposite();
            active.elapsed = Duration::ZERO;
            active.tracking = false;
        }
        timeout
    }

    /// Call when leaving LookLeftAndRightOf, including damping or another pattern.
    pub fn reset(&mut self) {
        self.active = None;
    }
}
