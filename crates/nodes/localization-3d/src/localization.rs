use color_eyre::{Result, eyre::eyre};

use crate::{
    diagnostics::SolveDiagnostics, estimator::Estimator, heading::HeadingReference,
    parameters::Localization3dParameters,
};
use booster::ImuState;
use coordinate_systems::{Local, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use ros_z::time::Time;
use types::camera_geometry::CameraGeometry;
use types::{
    field_dimensions::FieldDimensions,
    localization::{LocalizationEstimate, LocalizationState, LocalizationStatus},
    time_wrapper::TimeWrapper,
    visual_localization::{VisualAssociationSource, VisualLocalizationFrame},
    visual_odometry::VisualOdometer,
};

pub struct SolveOutput {
    pub estimate: Option<LocalizationEstimate>,
    pub diagnostics: SolveDiagnostics,
}

#[cfg(test)]
mod flight_recording_tests;
#[cfg(test)]
mod recording_tests;
#[cfg(test)]
mod recovery_tests;

/// Owns the graph and accepted lifecycle. The ROSZ node and deterministic runner
/// use the same operations; neither maintains another pending-measurement queue.
pub struct Localization {
    estimator: Estimator,
    parameters: Localization3dParameters,
    status: LocalizationStatus,
    latest_visual: Option<Time>,
    pending_visual: Option<Time>,
    last_solve: Option<Time>,
    heading_reference: Option<HeadingReference>,
    pending_bootstrap: Option<TimeWrapper<VisualLocalizationFrame>>,
}

impl Localization {
    pub(crate) fn has_measurement_gap(&self, time: Time) -> bool {
        time > self
            .estimator
            .latest_time()
            .saturating_add(self.parameters.timing.optimization_window)
    }
    pub fn new(
        time: Time,
        epoch: u64,
        parameters: &Localization3dParameters,
        field: &FieldDimensions,
        camera: &CameraGeometry,
        initial_pose: Isometry3<Robot, Local>,
    ) -> Result<Self> {
        parameters.validate().map_err(|message| eyre!(message))?;
        if !camera.intrinsics.is_valid()
            || !initial_pose
                .inner
                .to_homogeneous()
                .iter()
                .all(|v| v.is_finite())
        {
            return Err(eyre!("invalid localization initialization geometry"));
        }
        Ok(Self {
            estimator: Estimator::new(
                time,
                epoch,
                initial_pose,
                &camera.intrinsics,
                parameters.clone(),
                field,
            )?,
            parameters: parameters.clone(),
            status: LocalizationStatus {
                time,
                epoch,
                generation: 0,
                state: LocalizationState::Startup,
                heading: None,
            },
            latest_visual: None,
            pending_visual: None,
            last_solve: None,
            heading_reference: None,
            pending_bootstrap: None,
        })
    }

    pub fn status(&self) -> LocalizationStatus {
        LocalizationStatus {
            heading: self
                .heading_reference
                .map(|heading| heading.snapshot(self.parameters.max_heading_error)),
            ..self.status
        }
    }
    pub fn set_parameters(&mut self, parameters: &Localization3dParameters) -> Result<()> {
        self.parameters
            .validate_update(parameters)
            .map_err(|message| eyre!(message))?;
        self.estimator.update_parameters(parameters.clone())?;
        self.parameters = parameters.clone();
        Ok(())
    }

    pub(crate) fn max_camera_gap(&self) -> std::time::Duration {
        self.parameters.timing.max_camera_gap
    }

    pub fn deadline(&self) -> Option<Time> {
        if self.status.state != LocalizationState::Tracking {
            return None;
        }
        Some(
            self.last_solve?
                .saturating_add(self.parameters.tracking_timeout)
                .min(
                    self.latest_visual?
                        .saturating_add(self.parameters.visual_tracking_timeout),
                ),
        )
    }

    pub fn advance_time(&mut self, now: Time) {
        if self.deadline().is_some_and(|deadline| deadline <= now) {
            if let Some(heading) = self.heading_reference.as_mut() {
                heading.interrupt_tracking();
            }
            self.status = LocalizationStatus {
                time: now,
                state: LocalizationState::LostTrack,
                ..self.status
            };
        }
    }

    pub fn ingest_imu(&mut self, time: Time, imu: ImuState) -> Result<bool> {
        self.estimator.ingest_imu(time, imu)
    }

    pub fn ingest_kinematics(&mut self, sample: TimeWrapper<RobotKinematics>) -> Result<bool> {
        self.estimator.ingest_kinematics(sample)
    }

    pub fn ingest_kinematic_odometry(
        &mut self,
        sample: types::odometry::KinematicOdometryDelta,
    ) -> Result<bool> {
        self.estimator.ingest_kinematic_odometry(sample)
    }

    pub fn ingest_visual_odometry(
        &mut self,
        sample: VisualOdometer,
        previous: Option<&CameraGeometry>,
        current: Option<&CameraGeometry>,
    ) -> Result<bool> {
        self.estimator
            .ingest_visual_odometry(sample, previous, current)
    }

    pub fn ingest_visual_localization_frame(
        &mut self,
        frame: TimeWrapper<VisualLocalizationFrame>,
    ) -> Result<bool> {
        let time = frame.time;
        if frame.inner.epoch != self.status.epoch
            || frame.inner.generation != self.status.generation
        {
            return Ok(false);
        }
        // Recovery requires evidence acquired after loss, not delayed pre-loss work.
        if self.status.state == LocalizationState::LostTrack && time <= self.status.time {
            return Ok(false);
        }
        if self.status.state == LocalizationState::Startup
            || frame.inner.source == VisualAssociationSource::Global
        {
            match self.status.state {
                // A result can arrive after the LostTrack snapshot used by association.
                LocalizationState::Tracking => return Ok(false),
                LocalizationState::LostTrack | LocalizationState::Startup => {
                    if (self.status.state == LocalizationState::LostTrack
                        && self.heading_reference.is_none())
                        || !crate::alignment::valid_visual_frame(
                            &frame.inner,
                            &self.parameters.visual,
                        )
                        || self
                            .pending_bootstrap
                            .as_ref()
                            .is_some_and(|old| old.time >= time)
                    {
                        return Ok(false);
                    }
                    self.pending_bootstrap = Some(frame);
                    return Ok(true);
                }
            }
        }
        let inserted = self.estimator.ingest_visual(frame)?;
        if inserted && self.latest_visual.is_none_or(|old| time > old) {
            self.pending_visual = Some(self.pending_visual.map_or(time, |old| old.max(time)));
        }
        Ok(inserted)
    }

    pub fn solve(&mut self, now: Time) -> SolveOutput {
        let started = std::time::Instant::now();
        self.advance_time(now);
        // Incorporate this batch's motion before carrying its trajectory/velocity
        // into a bootstrap candidate, especially during moving startup.
        let solved = self
            .estimator
            .solve_with_heading(self.heading_reference.as_ref());
        if solved.visual_rejected {
            self.pending_visual = None;
        }
        let solved = if solved.motion_invalid {
            self.rebuild_local_motion(now).unwrap_or(solved)
        } else {
            solved
        };
        let solved = self.try_bootstrap(now).unwrap_or(solved);
        if solved.estimate.is_none() {
            self.pending_visual = None;
        }
        let estimate = solved.estimate.map(|mut estimate| {
            let visual_time = self.pending_visual.or(self.latest_visual);
            let visual_valid = visual_time.is_some_and(|time| {
                time.saturating_add(self.parameters.visual_tracking_timeout) > now
                    && (self.status.state != LocalizationState::LostTrack
                        || time > self.status.time)
            });
            // An initialization candidate is not field localization until the
            // optimized frame agrees with its actual pixel observations.
            let field_valid = estimate.robot_to_field.is_some()
                && crate::alignment::valid_visual_rms(
                    self.estimator.accepted_visual_rms(),
                    &self.parameters.visual,
                )
                && visual_time.is_some_and(|time| self.estimator.attitude_at(time).is_some());
            if solved.converged && visual_valid && field_valid {
                let continuing_tracking = self.status.state == LocalizationState::Tracking;
                if let Some(time) = self.pending_visual
                    && let Some((field, imu)) = self
                        .estimator
                        .field_heading_at(time)
                        .zip(self.estimator.attitude_at(time))
                {
                    match self.heading_reference.as_mut() {
                        None => {
                            self.heading_reference = Some(HeadingReference::new(time, field, imu))
                        }
                        Some(reference) if continuing_tracking => reference.observe_tracking(
                            time,
                            field,
                            imu,
                            self.parameters.max_heading_reference_drift_per_second,
                        ),
                        Some(reference) => reference.interrupt_tracking(),
                    }
                }
                self.latest_visual = visual_time;
                self.pending_visual = None;
                if self.status.state != LocalizationState::Tracking {
                    self.status = LocalizationStatus {
                        time: now,
                        state: LocalizationState::Tracking,
                        ..self.status
                    };
                }
            }
            if self.status.state == LocalizationState::Startup {
                estimate.robot_to_field = None;
            }
            if solved.converged {
                self.last_solve = Some(estimate.time);
            }
            estimate
        });
        self.advance_time(now);
        let mut diagnostics = solved.diagnostics;
        diagnostics.estimation_duration = started.elapsed();
        SolveOutput {
            estimate,
            diagnostics,
        }
    }

    fn try_bootstrap(&mut self, now: Time) -> Option<crate::estimator::SolveResult> {
        let frame = self.pending_bootstrap.as_ref()?;
        if self.status.state == LocalizationState::Tracking
            || (self.status.state == LocalizationState::LostTrack && frame.time <= self.status.time)
            || frame.time > now
            || frame
                .time
                .saturating_add(self.parameters.visual_tracking_timeout)
                <= now
        {
            self.pending_bootstrap = None;
            return None;
        }
        // Source streams arrive independently. Keep the newest recovery image until
        // its IMU bracket is available (or the normal freshness limit expires).
        self.estimator.attitude_at(frame.time)?;
        let frame = self.pending_bootstrap.take()?;
        let time = frame.time;
        let candidate = self
            .estimator
            .bootstrap_candidate(frame, self.heading_reference.as_ref());
        let predicted_height = self.estimator.height_prediction(time);
        let mut candidate = candidate
            .inspect_err(|error| tracing::warn!(%error, "discarding recovery candidate"))
            .ok()??;
        let solved = candidate.solve_with_heading(self.heading_reference.as_ref());
        if !solved.converged
            || solved
                .estimate
                .as_ref()
                .is_none_or(|estimate| estimate.robot_to_field.is_none())
            || !crate::alignment::valid_visual_rms(
                candidate.accepted_visual_rms(),
                &self.parameters.visual,
            )
        {
            return None;
        }
        if let Some(prediction) = predicted_height {
            let candidate_height = candidate.height_prediction(time)?;
            if !prediction.agrees_with(&candidate_height, self.parameters.recovery_height_gate) {
                tracing::warn!(
                    predicted_height = prediction.height,
                    recovered_height = candidate_height.height,
                    "discarding height-inconsistent recovery"
                );
                return None;
            }
        }
        // Publish only the validated graph, evaluated through the latest motion sample.
        self.status.generation = candidate.generation();
        self.estimator = candidate;
        self.pending_visual = Some(time);
        Some(solved)
    }

    fn rebuild_local_motion(&mut self, now: Time) -> Option<crate::estimator::SolveResult> {
        let mut candidate = self
            .estimator
            .motion_candidate()
            .inspect_err(|error| tracing::warn!(%error, "motion reconstruction failed"))
            .ok()??;
        let mut solved = candidate.solve_with_heading(None);
        solved.estimate.as_ref()?;
        self.estimator = candidate;
        self.pending_visual = None;
        self.latest_visual = None;
        if self.status.state == LocalizationState::Tracking {
            self.status = LocalizationStatus {
                time: now,
                state: LocalizationState::LostTrack,
                ..self.status
            };
        }
        if let Some(reference) = self.heading_reference.as_mut() {
            reference.interrupt_tracking();
        }
        solved.diagnostics.motion_rebuilt = true;
        Some(solved)
    }
}
