//! One-axis, entirely local trajectory calculation. No fixed update period is assumed.

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use rsruckig::{
    error::{RuckigError, RuckigErrorHandler},
    prelude::*,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct KinematicState {
    pub position: f64,
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    pub position: [f64; 2],
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

// Preserve validation errors while letting calculate return its typed failure status.
// ThrowErrorHandler would turn ErrorPositionalLimits into an untyped error string.
#[derive(Default, Debug)]
struct CalculationStatus;
impl RuckigErrorHandler for CalculationStatus {
    fn handle_validation_error(message: &str) -> Result<(), RuckigError> {
        Err(RuckigError::ValidationError(message.into()))
    }
    fn handle_calculator_error(_: &str) -> Result<(), RuckigError> {
        Ok(())
    }
}

pub(super) struct AxisStep {
    pub reference: KinematicState,
    pub effective_target: f32,
    pub recovered: bool,
}

pub(super) struct AxisGenerator {
    generator: Ruckig<1, CalculationStatus>,
    trajectory: Trajectory<1>,
}

impl Default for AxisGenerator {
    fn default() -> Self {
        Self {
            // calculate + at_time use the supplied elapsed time, not delta_time.
            generator: Ruckig::new(None, 0.0),
            trajectory: Trajectory::new(None),
        }
    }
}

impl AxisGenerator {
    /// Constrain the goal and advance, recovering from measurements only when
    /// the previous reference cannot produce a position-bounded trajectory.
    pub fn step(
        &mut self,
        start: KinematicState,
        requested: f32,
        measured_position: f32,
        limits: Limits,
        elapsed: f64,
    ) -> Result<AxisStep> {
        let [minimum, maximum] = limits.position;
        let effective_target = f64::from(requested).clamp(minimum, maximum);
        if (minimum..=maximum).contains(&start.position)
            && let Some(reference) = self.advance(start, effective_target, limits, elapsed)?
        {
            return Ok(AxisStep {
                reference,
                effective_target: effective_target as f32,
                recovered: false,
            });
        }

        let reference = self.recover(measured_position, effective_target, limits)?;
        Ok(AxisStep {
            reference,
            effective_target: effective_target as f32,
            recovered: true,
        })
    }

    fn recover(
        &mut self,
        measured_position: f32,
        target: f64,
        limits: Limits,
    ) -> Result<KinematicState> {
        // Outside the viable state set: position bounds win over derivative
        // continuity. Start a new trajectory at the bounded measurement, at rest.
        let start = KinematicState {
            position: f64::from(measured_position).clamp(limits.position[0], limits.position[1]),
            ..Default::default()
        };
        self.advance(start, target, limits, 0.0)?
            .ok_or_else(|| eyre!("no bounded head trajectory from rest"))
    }

    /// None means no position-bounded trajectory could be found from this state.
    /// Other failures are errors, never a silently accepted partial trajectory.
    fn advance(
        &mut self,
        state: KinematicState,
        target: f64,
        limits: Limits,
        elapsed: f64,
    ) -> Result<Option<KinematicState>> {
        let mut input = InputParameter::<1>::new(None);
        input.current_position[0] = state.position;
        input.current_velocity[0] = state.velocity;
        input.current_acceleration[0] = state.acceleration;
        input.target_position[0] = target;
        input.max_velocity[0] = limits.velocity;
        input.max_acceleration[0] = limits.acceleration;
        input.max_jerk[0] = limits.jerk;
        input.min_position = Some(DataArrayOrVec::Stack([limits.position[0]]));
        input.max_position = Some(DataArrayOrVec::Stack([limits.position[1]]));
        let result = self.generator.calculate(&input, &mut self.trajectory)?;
        if result == RuckigResult::ErrorPositionalLimits {
            return Ok(None);
        }
        ensure!(
            matches!(result, RuckigResult::Working | RuckigResult::Finished),
            "head trajectory calculation failed: {result:?}"
        );
        let extrema = &self.trajectory.get_position_extrema()[0];
        // Check the entire movement, including reversals between request samples.
        if extrema.min < limits.position[0] - 1e-8 || extrema.max > limits.position[1] + 1e-8 {
            return Ok(None);
        }
        let mut position = DataArrayOrVec::Stack([0.0]);
        let mut velocity = DataArrayOrVec::Stack([0.0]);
        let mut acceleration = DataArrayOrVec::Stack([0.0]);
        let mut jerk = DataArrayOrVec::Stack([0.0]);
        self.trajectory.at_time(
            elapsed.min(self.trajectory.get_duration()),
            &mut Some(&mut position),
            &mut Some(&mut velocity),
            &mut Some(&mut acceleration),
            &mut Some(&mut jerk),
            &mut None,
        );
        ensure!(
            [position[0], velocity[0], acceleration[0], jerk[0]]
                .into_iter()
                .all(f64::is_finite),
            "head trajectory contains non-finite values"
        );
        Ok(Some(KinematicState {
            // Only absorb numerical roundoff; substantive excursions were rejected above.
            position: position[0].clamp(limits.position[0], limits.position[1]),
            velocity: velocity[0],
            acceleration: acceleration[0],
            jerk: jerk[0],
        }))
    }
}
