//! Request resolution and coordination, independent of ROS interfaces and logging.

use std::time::Duration;

use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use coordinate_systems::Ground;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Point2, point};
use ros_z::time::Time;
use types::{
    field_dimensions::{FieldDimensions, GlobalFieldSide},
    joint_limits::JointLimits,
    motion_command::{HeadMotion, ImageRegion},
    support_foot::Side,
};

use crate::{
    joint_control::{
        HeadObservation, JointControlOutput, JointController, JointTarget, MotionProgress,
    },
    look_at::{GazeGeometry, LookAtError, look_at},
    parameters::Parameters,
    patterns::{GlanceState, GlanceTimeout, ScanKind, ScanState, ScanTimeout},
};

/// A consistent request-time snapshot. Geometry and field dimensions are only
/// required by the motions that use them; joint limits are always required.
pub struct HeadContext<'a> {
    pub geometry: Option<GazeGeometry<'a>>,
    pub joint_limits: Option<&'a JointLimits>,
    pub field_dimensions: Option<&'a FieldDimensions>,
    pub field_side: Option<GlobalFieldSide>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    MissingGeometry,
    MissingFieldDimensions,
    InvalidFieldWidth,
    Geometry(LookAtError),
}

pub struct HeadOutput {
    /// The measurement snapshot used to generate commands and diagnostics.
    pub observation: HeadObservation,
    pub joint_control: JointControlOutput,
    pub hold_reason: Option<HoldReason>,
    pub scan_timeout: Option<ScanTimeout>,
    pub glance_timeout: Option<GlanceTimeout>,
    pub injected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Direct,
    Scan(ScanKind),
    Glance,
    Damping,
    Injected,
}

struct TimedObservation {
    value: HeadObservation,
    time: Time,
}

#[derive(Default)]
pub struct HeadController {
    observation: Option<TimedObservation>,
    mode: Option<Mode>,
    last_evaluation: Option<Time>,
    reference_position: Option<HeadJoints<f32>>,
    hold_target: Option<HeadJoints<f32>>,
    scan_progress: Option<MotionProgress>,
    scan: ScanState,
    glance: GlanceState,
    joint_control: JointController,
}

impl HeadController {
    /// Discard measurements when the input adapter cannot decode a LowState sample.
    pub fn invalidate_observation(&mut self) {
        self.observation = None;
    }

    /// Receive every LowState-derived head observation without generating commands.
    /// Invalid observations invalidate the cache instead of retaining older input.
    pub fn observe(&mut self, observation: HeadObservation, now: Time) -> Result<()> {
        let rollback = self
            .observation
            .as_ref()
            .is_some_and(|previous| now < previous.time);
        self.observation = None;
        observation.validate()?;
        if rollback {
            self.reset_motion();
        }
        self.observation = Some(TimedObservation {
            value: observation,
            time: now,
        });
        Ok(())
    }

    /// Evaluate only when central motion requests commands. Errors produce no output;
    /// geometry failures produce a bounded hold command and a structured reason.
    pub fn evaluate(
        &mut self,
        request: &HeadMotion,
        context: &HeadContext<'_>,
        parameters: &Parameters,
        now: Time,
    ) -> Result<HeadOutput> {
        let result = self.evaluate_request(request, context, parameters, now);
        if result.is_err() {
            self.scan_progress = None;
            self.glance.advance(None);
        }
        result.wrap_err_with(|| format!("failed to evaluate head request {request:?}"))
    }

    fn evaluate_request(
        &mut self,
        request: &HeadMotion,
        context: &HeadContext<'_>,
        parameters: &Parameters,
        now: Time,
    ) -> Result<HeadOutput> {
        parameters.validate().map_err(|reason| eyre!(reason))?;
        let observation = self.current_observation(now, parameters.maximum_observation_age)?;
        let joints = context
            .joint_limits
            .ok_or_else(|| eyre!("head joint limits are unavailable"))?;
        joints.validate().map_err(|reason| eyre!(reason))?;
        let mode = mode_for(request, parameters.injected_head_joints.is_some());
        self.prepare_mode(mode, now, parameters.joint_control.reseed_after);
        let reference = self.reference_position.unwrap_or(observation.positions);
        let resolution = self.resolve(request, context, parameters, reference, now)?;
        let output = self.joint_control.update(
            resolution.target,
            &observation,
            &parameters.joint_control,
            joints,
            now,
        )?;
        let glance_timeout =
            self.record_output(mode, resolution.hold_reason.is_some(), &output, now);
        Ok(HeadOutput {
            observation,
            joint_control: output,
            hold_reason: resolution.hold_reason,
            scan_timeout: resolution.scan_timeout,
            glance_timeout,
            injected: mode == Mode::Injected,
        })
    }

    fn record_output(
        &mut self,
        mode: Mode,
        holding: bool,
        output: &JointControlOutput,
        now: Time,
    ) -> Option<GlanceTimeout> {
        let glance_timeout = if mode == Mode::Glance {
            self.glance.advance(if !holding {
                output.progress.as_ref()
            } else {
                None
            })
        } else {
            None
        };
        self.scan_progress = if matches!(mode, Mode::Scan(_)) {
            output.progress
        } else {
            None
        };
        self.reference_position = if mode == Mode::Damping {
            None
        } else {
            Some(HeadJoints {
                yaw: output.reference.yaw.position as f32,
                pitch: output.reference.pitch.position as f32,
            })
        };
        self.last_evaluation = Some(now);
        glance_timeout
    }

    fn current_observation(&self, now: Time, maximum_age: Duration) -> Result<HeadObservation> {
        let observation = self
            .observation
            .as_ref()
            .ok_or_else(|| eyre!("head observation is unavailable"))?;
        ensure!(
            now >= observation.time,
            "head observation is ahead of the request clock"
        );
        let age = now.duration_since(observation.time);
        ensure!(
            age <= maximum_age,
            "head observation is stale: age={age:?}, maximum_age={maximum_age:?}"
        );
        Ok(observation.value)
    }

    fn prepare_mode(&mut self, mode: Mode, now: Time, reseed_after: Duration) {
        if self
            .last_evaluation
            .is_some_and(|last| now < last || now.duration_since(last) > reseed_after)
        {
            self.reset_motion();
        }
        if self.mode != Some(mode) {
            self.scan.reset();
            self.glance.reset();
            self.scan_progress = None;
            self.hold_target = None;
        }
        self.mode = Some(mode);
    }

    fn resolve(
        &mut self,
        request: &HeadMotion,
        context: &HeadContext<'_>,
        parameters: &Parameters,
        reference: HeadJoints<f32>,
        now: Time,
    ) -> Result<Resolution> {
        // Preserve the existing explicit debug override; it still uses joint control.
        if let Some(position) = parameters.injected_head_joints {
            self.hold_target = None;
            return Ok(Resolution::motion(move_to(
                position,
                parameters.direct_travel_speed,
            )));
        }
        let resolution = match *request {
            HeadMotion::ZeroAngles => {
                self.hold_target = None;
                Resolution::motion(move_to(
                    HeadJoints::fill(0.0),
                    parameters.direct_travel_speed,
                ))
            }
            HeadMotion::Damping => {
                self.hold_target = None;
                Resolution::motion(JointTarget::Damping)
            }
            HeadMotion::Center {
                image_region_target,
            } => self.center(image_region_target, context, parameters, reference),
            HeadMotion::LookAt {
                target,
                height_above_ground,
                image_region_target,
            } => {
                let angles = gaze(
                    target,
                    height_above_ground,
                    image_region_target,
                    context,
                    parameters,
                    reference,
                );
                self.gaze_motion(angles, parameters.direct_travel_speed, reference)
            }
            HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                let kind = if matches!(request, HeadMotion::LookAround) {
                    ScanKind::LookAround
                } else {
                    ScanKind::SearchForLostBall
                };
                let side = if context.field_side == Some(GlobalFieldSide::Away) {
                    Side::Right
                } else {
                    Side::Left
                };
                let scan =
                    self.scan
                        .update(kind, side, self.scan_progress.as_ref(), parameters, now)?;
                Resolution {
                    target: scan.target,
                    hold_reason: None,
                    scan_timeout: scan.timeout,
                }
            }
            HeadMotion::LookLeftAndRightOf {
                target,
                height_above_ground,
            } => {
                let angles = self
                    .glance
                    .update(target, parameters, now)
                    .map_err(|_| HoldReason::Geometry(LookAtError::InvalidTarget))
                    .and_then(|offset| {
                        gaze(
                            offset.position,
                            height_above_ground,
                            offset.image_region,
                            context,
                            parameters,
                            reference,
                        )
                    });
                self.gaze_motion(angles, parameters.glance.travel_speed, reference)
            }
        };
        Ok(resolution)
    }

    fn center(
        &mut self,
        image_region: ImageRegion,
        context: &HeadContext<'_>,
        parameters: &Parameters,
        reference: HeadJoints<f32>,
    ) -> Resolution {
        let angles = context
            .field_dimensions
            .ok_or(HoldReason::MissingFieldDimensions)
            .and_then(|field| {
                if !field.width.is_finite() || field.width <= 0.0 {
                    return Err(HoldReason::InvalidFieldWidth);
                }
                gaze(
                    point![field.width / 2.0, 0.0],
                    0.0,
                    image_region,
                    context,
                    parameters,
                    reference,
                )
            });
        self.gaze_motion(angles, parameters.direct_travel_speed, reference)
    }

    fn gaze_motion(
        &mut self,
        angles: Result<HeadJoints<f32>, HoldReason>,
        travel_speed: HeadJoints<f32>,
        reference: HeadJoints<f32>,
    ) -> Resolution {
        match angles {
            Ok(position) => {
                self.hold_target = None;
                Resolution::motion(move_to(position, travel_speed))
            }
            Err(reason) => {
                let position = *self.hold_target.get_or_insert(reference);
                Resolution {
                    target: move_to(position, travel_speed),
                    hold_reason: Some(reason),
                    scan_timeout: None,
                }
            }
        }
    }

    /// Explicitly reset all motion state and seed the measurement cache.
    pub fn reset(&mut self, observation: HeadObservation, now: Time) -> Result<()> {
        self.reset_motion();
        self.observe(observation, now)
    }

    fn reset_motion(&mut self) {
        self.mode = None;
        self.last_evaluation = None;
        self.reference_position = None;
        self.hold_target = None;
        self.scan_progress = None;
        self.scan.reset();
        self.glance.reset();
        self.joint_control = JointController::default();
    }
}

struct Resolution {
    target: JointTarget,
    hold_reason: Option<HoldReason>,
    scan_timeout: Option<ScanTimeout>,
}

impl Resolution {
    fn motion(target: JointTarget) -> Self {
        Self {
            target,
            hold_reason: None,
            scan_timeout: None,
        }
    }
}

fn move_to(position: HeadJoints<f32>, travel_speed: HeadJoints<f32>) -> JointTarget {
    JointTarget::MoveTo {
        position,
        travel_speed,
    }
}

fn gaze(
    target: Point2<Ground>,
    height: f32,
    image_region: ImageRegion,
    context: &HeadContext<'_>,
    parameters: &Parameters,
    reference: HeadJoints<f32>,
) -> Result<HeadJoints<f32>, HoldReason> {
    let geometry = context
        .geometry
        .as_ref()
        .ok_or(HoldReason::MissingGeometry)?;
    look_at(
        target,
        height,
        image_region,
        geometry,
        &parameters.image_region_parameters,
        reference,
    )
    .map_err(HoldReason::Geometry)
}

fn mode_for(request: &HeadMotion, injected: bool) -> Mode {
    if injected {
        return Mode::Injected;
    }
    match request {
        HeadMotion::LookAround => Mode::Scan(ScanKind::LookAround),
        HeadMotion::SearchForLostBall => Mode::Scan(ScanKind::SearchForLostBall),
        HeadMotion::LookLeftAndRightOf { .. } => Mode::Glance,
        HeadMotion::Damping => Mode::Damping,
        HeadMotion::ZeroAngles | HeadMotion::Center { .. } | HeadMotion::LookAt { .. } => {
            Mode::Direct
        }
    }
}

#[cfg(test)]
mod tests;
