use color_eyre::Result;
use coordinate_systems::Robot;
use linear_algebra::{IntoTransform, Isometry3, Point3, Vector3};
use localization_fagra::variables::{FieldAlignment, PoseControl, TrajectoryState};
use nalgebra::{Matrix2, UnitQuaternion};
use projection::intrinsic::Intrinsic;
use ros_z::time::Time;
use types::{time_wrapper::TimeWrapper, visual_localization::VisualLocalizationFrame};

use super::{Estimator, control_keys};
use crate::{
    alignment::{seed_alignment, seed_recovery_alignment, valid_visual_frame},
    heading::HeadingReference,
};

/// Accepted, normalized observations. Replay bypasses stream admission, not factor construction.
#[derive(Clone)]
pub(super) enum MotionRecord {
    Imu {
        time: Time,
        angular_velocity: Vector3<Robot, f64>,
        attitude: linear_algebra::Orientation3<coordinate_systems::ImuReference, f64>,
        /// Raw specific force; calibration is applied when rebuilding intervals.
        force: Vector3<Robot, f64>,
    },
    Feet {
        time: Time,
        left: Point3<Robot, f64>,
        right: Point3<Robot, f64>,
    },
    Kinematic {
        previous_time: Time,
        time: Time,
        translation: linear_algebra::Vector2<coordinate_systems::Ground, f64>,
        information_root: Matrix2<f64>,
    },
    Visual {
        previous_time: Time,
        time: Time,
        transform: Isometry3<Robot, Robot, f64>,
    },
}

impl MotionRecord {
    fn start(&self) -> Time {
        match self {
            Self::Imu { time, .. } | Self::Feet { time, .. } => *time,
            Self::Kinematic { previous_time, .. } | Self::Visual { previous_time, .. } => {
                *previous_time
            }
        }
    }

    fn end(&self) -> Time {
        match self {
            Self::Imu { time, .. }
            | Self::Feet { time, .. }
            | Self::Kinematic { time, .. }
            | Self::Visual { time, .. } => *time,
        }
    }

    fn insert(self, estimator: &mut Estimator) -> Result<()> {
        match self {
            Self::Imu {
                time,
                angular_velocity,
                attitude,
                force,
            } => estimator.insert_imu(time, angular_velocity, attitude, force),
            Self::Feet { time, left, right } => estimator.insert_feet(time, left, right),
            Self::Kinematic {
                previous_time,
                time,
                translation,
                information_root,
            } => estimator.insert_kinematic_odometry(
                previous_time,
                time,
                translation,
                information_root,
            ),
            Self::Visual {
                previous_time,
                time,
                transform,
            } => estimator.insert_visual_odometry(previous_time, time, transform),
        }
    }
}

impl Estimator {
    /// Resolve the global match's half-turn convention, then update the retained graph.
    /// Its marginalized history, alignment and motion factors must explain recovery.
    pub(crate) fn ingest_recovery(
        &mut self,
        mut frame: TimeWrapper<VisualLocalizationFrame>,
        heading: &HeadingReference,
    ) -> Result<bool> {
        let Some(attitude) = self.attitude_at(frame.time) else {
            return Ok(false);
        };
        let (roll, pitch, _) = attitude.euler_angles();
        let Some((pose, alignment)) = seed_recovery_alignment(
            &mut frame.inner,
            linear_algebra::Rotation3::from_euler_angles(roll as f32, pitch as f32, 0.0),
            types::localization::HeadingConstraint {
                expected: heading.expected(attitude),
                max_error: self.parameters.max_heading_error,
            },
            &self.parameters.visual,
        ) else {
            return Ok(false);
        };
        let time = frame.time;
        let inserted = self.ingest_visual(frame)?;
        if inserted {
            self.recovery_seed = Some((time, (alignment.to_3d() * pose).inner.cast()));
        }
        Ok(inserted)
    }

    pub(super) fn apply_recovery_seed(&mut self) -> Result<()> {
        let Some((time, field_pose)) = self.recovery_seed.take() else {
            return Ok(());
        };
        let Some(alignment) = self.alignment else {
            return Ok(());
        };
        let (segment, tau) = self.segment_and_tau(time)?;
        let old = self
            .spline(control_keys(&self.controls, segment)?)?
            .pose(tau)?
            .inner;
        let local_pose = self
            .graph
            .get(alignment)?
            .local_to_field
            .to_3d()
            .inner
            .inverse()
            * field_pose;
        let correction = local_pose * old.inverse();
        let first = *self.controls.first_key_value().unwrap().0;
        // This only seeds optimization; every prior and motion factor remains in force.
        for (&index, &key) in &self.controls {
            let weight = ((index - first) as f64 / (segment + 1 - first) as f64).clamp(0.0, 1.0);
            let mut control = self.graph.get(key)?.clone();
            let target = correction * control.pose.inner;
            control.pose.inner.translation.vector = control
                .pose
                .inner
                .translation
                .vector
                .lerp(&target.translation.vector, weight);
            control.pose.inner.rotation =
                control.pose.inner.rotation.slerp(&target.rotation, weight);
            self.graph.set(key, control)?;
        }
        Ok(())
    }

    pub(super) fn accept_motion(&mut self, record: MotionRecord) -> Result<bool> {
        record.clone().insert(self)?;
        self.commit_time(record.end());
        let first = (self.oldest_window_segment() - 1).max(0);
        let start =
            Time::from_nanos(self.origin.as_nanos() + first * self.parameters.timing.knot_ns());
        if start > self.history_start {
            self.motion_history.retain(|record| record.start() >= start);
            // Preserve the source sample needed to interpolate the left boundary.
            if let Some((&before, _)) = self.attitudes.range(..=start).next_back() {
                self.attitudes.retain(|time, _| *time >= before);
            }
            self.history_start = start;
        }
        self.motion_history.push(record);
        Ok(true)
    }

    /// Rebootstrap recent motion without carrying field-conditioned marginal priors.
    /// The active estimator is untouched until Localization validates this candidate.
    pub(crate) fn bootstrap_candidate(
        &self,
        mut frame: TimeWrapper<VisualLocalizationFrame>,
        heading: Option<&HeadingReference>,
    ) -> Result<Option<Self>> {
        let Some((segment, tau)) = self.check_time(frame.time, "global recovery")? else {
            return Ok(None);
        };
        if frame.inner.epoch != self.epoch
            || frame.inner.generation != self.generation
            || !valid_visual_frame(&frame.inner, &self.parameters.visual)
            || frame.time > self.latest_time
        {
            return Ok(None);
        }
        let Some(attitude) = self.attitude_at(frame.time) else {
            return Ok(None);
        };
        let old = self
            .spline(control_keys(&self.controls, segment)?)?
            .state(tau)?;
        // IMU tilt at exposure, with the existing Local heading convention.
        let (roll, pitch, _) = attitude.euler_angles();
        let yaw = old.pose.inner.rotation.euler_angles().2;
        let robot_to_local = linear_algebra::Rotation3::wrap(
            UnitQuaternion::from_euler_angles(roll, pitch, yaw).cast(),
        );
        let seed = match heading {
            Some(heading) => seed_recovery_alignment(
                &mut frame.inner,
                robot_to_local,
                types::localization::HeadingConstraint {
                    expected: heading.expected(attitude),
                    max_error: self.parameters.max_heading_error,
                },
                &self.parameters.visual,
            ),
            None => seed_alignment(
                robot_to_local,
                frame.inner.robot_to_camera,
                frame.inner.camera_intrinsic,
                &mut frame.inner.associations,
                true,
                &self.parameters.visual,
            ),
        };
        let Some((pose, seed)) = seed else {
            return Ok(None);
        };
        let alignment = FieldAlignment {
            local_to_field: seed.inner.cast().framed_transform(),
        };
        let correction = pose.inner.cast::<f64>() * old.pose.inner.inverse();
        let mut candidate = self.rebuild_motion(
            frame.time,
            TrajectoryState {
                pose: linear_algebra::Framed::wrap(pose.inner.cast()),
                velocity: Vector3::wrap(correction.rotation * old.velocity.inner),
            },
            &frame.inner.camera_intrinsic,
            None,
        )?;
        candidate.generation = self.generation.wrapping_add(1);
        candidate.alignment = Some(candidate.graph.add(alignment));
        for segment in candidate.segments() {
            candidate.add_containment(segment)?;
        }
        frame.inner.generation = candidate.generation;
        if !candidate.ingest_visual(frame)? {
            return Ok(None);
        }
        Ok(Some(candidate))
    }

    fn rebuild_motion(
        &self,
        time: Time,
        anchor: TrajectoryState,
        intrinsics: &Intrinsic,
        height_sigma: Option<f64>,
    ) -> Result<Self> {
        let mut candidate = Self::empty(
            self.origin,
            self.epoch,
            intrinsics,
            self.parameters.clone(),
            self.field_half_extents,
        )?;
        candidate.options = self.options;
        candidate.generation = self.generation;
        candidate.initialize_biases(self.history_start, self.bias_at(self.history_start)?)?;
        let first = self.segment_and_tau(self.history_start)?.0;
        let last = self.segment_and_tau(self.latest_time)?.0;
        let reference_attitude = self
            .attitude_at(time)
            .ok_or_else(|| color_eyre::eyre::eyre!("missing anchor IMU attitude"))?;
        let yaw_offset =
            anchor.pose.inner.rotation.euler_angles().2 - reference_attitude.euler_angles().2;
        for index in first - 1..=last + 2 {
            let stamp = self.origin.as_nanos() + index * self.parameters.timing.knot_ns();
            let sample_time = Time::from_nanos(stamp);
            let attitude = self.attitude_at(sample_time).unwrap_or_else(|| {
                self.attitudes
                    .range(..=sample_time)
                    .next_back()
                    .or_else(|| self.attitudes.first_key_value())
                    .map_or(reference_attitude, |(_, a)| *a)
            });
            let (roll, pitch, yaw) = attitude.euler_angles();
            // Controls are initial guesses, not interpolation samples. Refit the
            // recorded motion instead of copying a possibly corrupted spline tail.
            let position = anchor.pose.inner.translation.vector
                + anchor.velocity.inner * ((stamp - time.as_nanos()) as f64 * 1e-9);
            let control = PoseControl {
                pose: linear_algebra::Framed::wrap(nalgebra::Isometry3::from_parts(
                    position.into(),
                    UnitQuaternion::from_euler_angles(roll, pitch, yaw + yaw_offset),
                )),
            };
            candidate
                .controls
                .insert(index, candidate.graph.add(control));
        }
        candidate.add_anchor(time, anchor, height_sigma)?;
        for segment in first..=last {
            candidate.add_motion_prior(segment, false)?;
        }
        for record in self.motion_history.iter().cloned() {
            record.insert(&mut candidate)?;
        }
        candidate.attitudes.clone_from(&self.attitudes);
        candidate.preintegration.restore_boundary(
            self.origin,
            self.history_start,
            &self.preintegration,
        );
        candidate.motion_history.clone_from(&self.motion_history);
        candidate.history_start = self.history_start;
        candidate.latest_time = self.latest_time;
        candidate.latest_kinematic_time = self.latest_kinematic_time;
        candidate.latest_vo_epoch = self.latest_vo_epoch;
        Ok(candidate)
    }
}
