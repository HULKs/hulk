//! Synchronized yaw/pitch trajectory calculation. No fixed update period is assumed.

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use kinematics::joints::head::HeadJoints;
use rsruckig::{
    error::{RuckigError, RuckigErrorHandler},
    prelude::*,
};

use super::JOINTS;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KinematicState {
    pub position: f64,
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

pub(super) struct TrajectoryGenerator {
    generator: Ruckig<2, CalculationStatus>,
    trajectory: Trajectory<2>,
    plan: Option<Plan>,
}

struct Plan {
    target: HeadJoints<f32>,
    limits: HeadJoints<Limits>,
    reference: HeadJoints<KinematicState>,
    time: f64,
}

impl Default for TrajectoryGenerator {
    fn default() -> Self {
        Self {
            // calculate + at_time use the supplied elapsed time, not delta_time.
            generator: Ruckig::new(None, 0.0),
            trajectory: Trajectory::new(None),
            plan: None,
        }
    }
}

impl TrajectoryGenerator {
    /// Plan both joints together. Recovery first tries reseeding a single joint,
    /// preserving the other's reference, before falling back to reseeding both.
    pub fn step(
        &mut self,
        start: HeadJoints<KinematicState>,
        requested: HeadJoints<f32>,
        measured_position: HeadJoints<f32>,
        limits: HeadJoints<Limits>,
        elapsed: f64,
    ) -> Result<HeadJoints<AxisStep>> {
        let target = HeadJoints {
            yaw: requested
                .yaw
                .clamp(limits.yaw.position[0] as f32, limits.yaw.position[1] as f32),
            pitch: requested.pitch.clamp(
                limits.pitch.position[0] as f32,
                limits.pitch.position[1] as f32,
            ),
        };
        // Sample an unchanged movement instead of recalculating its remaining
        // duration every request. Retargeting, new limits, or reseeding replans.
        if let Some(plan) = &self.plan
            && plan.target == target
            && plan.limits == limits
            && plan.reference == start
        {
            return self.output(target, limits, plan.time + elapsed, HeadJoints::fill(false));
        }
        self.plan = None;
        // Position-limit failures do not identify a joint. Try the smallest recovery
        // sets; each candidate must pass the complete synchronized trajectory check.
        for recovered in [
            HeadJoints::fill(false),
            HeadJoints {
                yaw: true,
                pitch: false,
            },
            HeadJoints {
                yaw: false,
                pitch: true,
            },
            HeadJoints::fill(true),
        ] {
            let recovering = recovered.yaw || recovered.pitch;
            let mut state = start;
            for joint in JOINTS {
                if recovered[joint] {
                    state[joint] = KinematicState {
                        position: f64::from(measured_position[joint])
                            .clamp(limits[joint].position[0], limits[joint].position[1]),
                        ..Default::default()
                    };
                }
            }
            if !self.calculate(state, target, limits)? {
                continue;
            }
            // Recovery emits the new starting reference; the unaffected joint keeps
            // its position and derivatives while the shared movement is replanned.
            return self.output(
                target,
                limits,
                if recovering { 0.0 } else { elapsed },
                recovered,
            );
        }
        Err(eyre!("no bounded synchronized head trajectory from rest"))
    }

    fn output(
        &mut self,
        target: HeadJoints<f32>,
        limits: HeadJoints<Limits>,
        time: f64,
        recovered: HeadJoints<bool>,
    ) -> Result<HeadJoints<AxisStep>> {
        let time = time.min(self.trajectory.get_duration());
        let reference = self.sample(time, limits)?;
        self.plan = Some(Plan {
            target,
            limits,
            reference,
            time,
        });
        let axis = |joint| AxisStep {
            reference: reference[joint],
            effective_target: target[joint],
            recovered: recovered[joint],
        };
        Ok(HeadJoints {
            yaw: axis(JOINTS[0]),
            pitch: axis(JOINTS[1]),
        })
    }

    /// False means no position-bounded trajectory could be found from this state.
    /// Other failures are errors, never a silently accepted partial trajectory.
    fn calculate(
        &mut self,
        state: HeadJoints<KinematicState>,
        target: HeadJoints<f32>,
        limits: HeadJoints<Limits>,
    ) -> Result<bool> {
        let mut input = InputParameter::<2>::new(None);
        // Ruckig tries a shared phase profile and falls back to time synchronization
        // when current derivatives are incompatible with a straight joint-space path.
        input.synchronization = Synchronization::Phase;
        for (axis, joint) in JOINTS.into_iter().enumerate() {
            let [minimum, maximum] = limits[joint].position;
            if !(minimum..=maximum).contains(&state[joint].position) {
                return Ok(false);
            }
            input.current_position[axis] = state[joint].position;
            input.current_velocity[axis] = state[joint].velocity;
            input.current_acceleration[axis] = state[joint].acceleration;
            input.target_position[axis] = f64::from(target[joint]);
            input.max_velocity[axis] = limits[joint].velocity;
            input.max_acceleration[axis] = limits[joint].acceleration;
            input.max_jerk[axis] = limits[joint].jerk;
        }
        input.min_position = Some(DataArrayOrVec::Stack([
            limits.yaw.position[0],
            limits.pitch.position[0],
        ]));
        input.max_position = Some(DataArrayOrVec::Stack([
            limits.yaw.position[1],
            limits.pitch.position[1],
        ]));
        align_phase_limits(&mut input);
        let result = self.generator.calculate(&input, &mut self.trajectory)?;
        if result == RuckigResult::ErrorPositionalLimits {
            return Ok(false);
        }
        ensure!(
            matches!(result, RuckigResult::Working | RuckigResult::Finished),
            "head trajectory calculation failed: {result:?}"
        );
        let extrema = self.trajectory.get_position_extrema();
        // Check both complete movements, including reversals between request samples.
        Ok(JOINTS.into_iter().enumerate().all(|(axis, joint)| {
            extrema[axis].min >= limits[joint].position[0] - 1e-8
                && extrema[axis].max <= limits[joint].position[1] + 1e-8
        }))
    }

    fn sample(
        &self,
        elapsed: f64,
        limits: HeadJoints<Limits>,
    ) -> Result<HeadJoints<KinematicState>> {
        let mut position = DataArrayOrVec::Stack([0.0; 2]);
        let mut velocity = DataArrayOrVec::Stack([0.0; 2]);
        let mut acceleration = DataArrayOrVec::Stack([0.0; 2]);
        let mut jerk = DataArrayOrVec::Stack([0.0; 2]);
        self.trajectory.at_time(
            elapsed.min(self.trajectory.get_duration()),
            &mut Some(&mut position),
            &mut Some(&mut velocity),
            &mut Some(&mut acceleration),
            &mut Some(&mut jerk),
            &mut None,
        );
        let mut reference = HeadJoints::default();
        for (axis, joint) in JOINTS.into_iter().enumerate() {
            ensure!(
                [
                    position[axis],
                    velocity[axis],
                    acceleration[axis],
                    jerk[axis]
                ]
                .into_iter()
                .all(f64::is_finite),
                "head trajectory contains non-finite values for {joint:?}"
            );
            reference[joint] = KinematicState {
                // Only absorb numerical roundoff; substantive excursions were rejected.
                position: position[axis]
                    .clamp(limits[joint].position[0], limits[joint].position[1]),
                velocity: velocity[axis],
                acceleration: acceleration[axis],
                jerk: jerk[axis],
            };
        }
        Ok(reference)
    }
}

/// A shared phase may be limited by yaw's jerk but pitch's speed. Ruckig copies
/// the slowest independent profile, so give both axes proportional limits when
/// their current derivatives permit a straight path.
fn align_phase_limits(input: &mut InputParameter<2>) {
    let distance = [
        input.target_position[0] - input.current_position[0],
        input.target_position[1] - input.current_position[1],
    ];
    let leading = usize::from(distance[1].abs() > distance[0].abs());
    if distance[leading].abs() < 1e-9 {
        return;
    }
    let ratio = distance.map(|delta| delta / distance[leading]);
    for (axis, ratio) in ratio.into_iter().enumerate() {
        if (input.current_velocity[axis] - ratio * input.current_velocity[leading]).abs() > 1e-9
            || (input.current_acceleration[axis] - ratio * input.current_acceleration[leading])
                .abs()
                > 1e-9
        {
            return;
        }
    }
    for limits in [
        &mut input.max_velocity,
        &mut input.max_acceleration,
        &mut input.max_jerk,
    ] {
        let shared = (0..2)
            .filter(|&axis| ratio[axis].abs() > 1e-9)
            .map(|axis| limits[axis] / ratio[axis].abs())
            .fold(f64::INFINITY, f64::min);
        for (axis, ratio) in ratio.into_iter().enumerate() {
            if ratio.abs() > 1e-9 {
                limits[axis] = limits[axis].min(shared * ratio.abs());
            }
        }
    }
}
