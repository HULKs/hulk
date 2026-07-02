use std::time::Duration;

use factrs::{containers::FactorBuilder, core::SO3, traits::Optimizer};

use crate::{
    factors::imu::{
        CurrentSplineOrientationFactor, IntervalGaussianProcessImuFactor, RelativeYawFactor,
        RollPitchPriorFactor, interpolate_measurement_orientation,
    },
    measurements::ImuMeasurement,
    symbols::State,
};

use super::VinsBackend;

const IMU_KINEMATICS_MIN_SPACING: Duration = Duration::from_millis(20); // 50Hz

impl VinsBackend {
    /// Ingests a batch of IMU measurements into the graph.
    /// Assumes the measurements are already sorted by time.
    pub(super) fn ingest_imu(&mut self, measurements: Vec<ImuMeasurement>) {
        let (Some(first), Some(last)) = (measurements.first(), measurements.last()) else {
            return;
        };
        let first_time = first.time;
        let last_time = last.time;
        let _ = self
            .interval_assigner
            .assign_or_initialize_interval(first_time);
        self.update_last_knot_time(last_time);

        let Some(last_interval_index) = self
            .interval_assigner
            .assign_or_initialize_interval(last_time)
        else {
            return;
        };
        self.init_intervals_through(last_interval_index);

        let kinematics_measurements =
            self.keep_imu_kinematics_measurements(measurements.iter().cloned());
        self.add_imu_kinematics_factors(kinematics_measurements);

        self.process_imu_attitude_measurements(measurements);
    }

    fn process_imu_attitude_measurements(&mut self, measurements: Vec<ImuMeasurement>) {
        let mut previous = self.last_imu_attitude_measurement.clone();

        for measurement in measurements {
            if let Some(previous) = previous.as_ref() {
                debug_assert!(
                    previous.time <= measurement.time,
                    "IMU measurements must be globally sorted by time"
                );
                self.add_imu_attitude_factors_between(previous, &measurement);
            } else {
                self.add_exact_imu_attitude_knot(&measurement);
            }

            previous = Some(measurement);
        }

        self.latest_imu_attitude_measurement = previous.clone();
        self.last_imu_attitude_measurement = previous;
    }

    fn add_exact_imu_attitude_knot(&mut self, measurement: &ImuMeasurement) {
        while let Some(knot_time) = self
            .interval_assigner
            .interval_start_time(self.next_imu_attitude_knot_index)
        {
            if knot_time > measurement.time {
                return;
            }

            if knot_time == measurement.time {
                self.add_imu_knot_orientation(
                    self.next_imu_attitude_knot_index,
                    interpolate_measurement_orientation(measurement, measurement, knot_time),
                );
            }
            self.next_imu_attitude_knot_index += 1;
        }
    }

    fn add_imu_attitude_factors_between(
        &mut self,
        previous: &ImuMeasurement,
        current: &ImuMeasurement,
    ) {
        while let Some(knot_time) = self
            .interval_assigner
            .interval_start_time(self.next_imu_attitude_knot_index)
        {
            if knot_time < previous.time {
                self.next_imu_attitude_knot_index += 1;
                continue;
            }
            if knot_time > current.time {
                return;
            }

            let measured_orientation =
                interpolate_measurement_orientation(previous, current, knot_time);
            self.add_imu_knot_orientation(self.next_imu_attitude_knot_index, measured_orientation);
            self.next_imu_attitude_knot_index += 1;
        }
    }

    fn add_imu_knot_orientation(&mut self, knot_index: u32, measured_orientation: SO3) {
        self.add_roll_pitch_prior_if_available(State(knot_index), measured_orientation.clone());

        if let Some(previous) = self.last_imu_knot_orientation.as_ref()
            && previous.index + 1 == knot_index
            && self.interval_states_available(previous.index)
        {
            self.add_relative_yaw_factor_if_available(
                previous.index,
                previous.orientation.clone(),
                measured_orientation.clone(),
            );
        }

        self.last_imu_knot_orientation = Some(ImuKnotOrientation {
            index: knot_index,
            orientation: measured_orientation,
        });
    }

    fn add_roll_pitch_prior_if_available(&mut self, state: State, measured_orientation: SO3) {
        if self.values.get(state).is_none() {
            return;
        }
        let graph = self.optimizer.graph_mut();
        if graph
            .factors_for_residual_mut::<RollPitchPriorFactor, _>(state)
            .next()
            .is_some()
        {
            return;
        }

        let residual =
            RollPitchPriorFactor::new(measured_orientation, self.config.roll_pitch_yaw_noise);
        let factor = FactorBuilder::new(residual, state).build();
        graph.add_factor(factor);
    }

    fn add_relative_yaw_factor_if_available(
        &mut self,
        interval_index: u32,
        measured_start_orientation: SO3,
        measured_end_orientation: SO3,
    ) {
        let keys = (State(interval_index), State(interval_index + 1));
        let graph = self.optimizer.graph_mut();
        if graph
            .factors_for_residual_mut::<RelativeYawFactor, _>(keys)
            .next()
            .is_some()
        {
            return;
        }

        let residual = RelativeYawFactor::new(
            measured_start_orientation,
            measured_end_orientation,
            self.config.roll_pitch_yaw_noise,
        );
        let factor = FactorBuilder::new(residual, keys).build();
        graph.add_factor(factor);
    }

    pub(super) fn add_current_spline_orientation_factor(&mut self) {
        let Some(current_measurement) = self.latest_imu_attitude_measurement.clone() else {
            return;
        };
        let Some(start_time) = self
            .interval_assigner
            .current_or_initialize_interval_start_time(current_measurement.time)
        else {
            return;
        };
        let Some(interval_index) = self
            .interval_assigner
            .assign_or_initialize_interval(start_time)
        else {
            return;
        };
        if !self.interval_states_available(interval_index) {
            return;
        }

        let Some(start_orientation) = self.last_imu_knot_orientation.as_ref() else {
            return;
        };
        if start_orientation.index != interval_index {
            return;
        }

        let end_time = start_time + self.config.knot_spacing;
        let residual = CurrentSplineOrientationFactor::new(
            start_orientation.orientation.clone(),
            &current_measurement,
            self.config.roll_pitch_yaw_noise,
            start_time,
            end_time,
        );
        let factor =
            FactorBuilder::new(residual, (State(interval_index), State(interval_index + 1)))
                .build();
        self.optimizer.graph_mut().add_factor(factor);
    }

    pub(super) fn remove_current_spline_orientation_factor(&mut self) {
        self.optimizer.graph_mut().remove_factors(|factor| {
            factor
                .residual_as::<CurrentSplineOrientationFactor>()
                .is_some()
        });
    }

    /// filter out measurements in order to keep graph from exploding with too many IMU kinematics factors when the frontend provides high-frequency IMU data
    fn keep_imu_kinematics_measurements(
        &mut self,
        measurements: impl IntoIterator<Item = ImuMeasurement>,
    ) -> Vec<ImuMeasurement> {
        let mut kept = Vec::new();

        for measurement in measurements {
            let keep = match self.last_imu_kinematics_measurement_time {
                None => true,
                Some(last) => measurement
                    .time
                    .duration_since(last)
                    .is_ok_and(|dt| dt >= IMU_KINEMATICS_MIN_SPACING),
            };

            if keep {
                self.last_imu_kinematics_measurement_time = Some(measurement.time);
                kept.push(measurement);
            }
        }

        kept
    }

    fn add_imu_kinematics_factors(&mut self, measurements: Vec<ImuMeasurement>) {
        if measurements.is_empty() {
            return;
        }

        let interval_groups = self.interval_groups(measurements, |measurement| measurement.time);

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, "imu kinematics") {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();

            if let Some(factor) = graph
                .factors_for_residual_mut::<IntervalGaussianProcessImuFactor, _>(keys)
                .next()
            {
                factor
                    .residual_as_mut::<IntervalGaussianProcessImuFactor>()
                    .expect("factor query must return matching residual")
                    .extend_measurements(group.measurements);
            } else {
                let residual = IntervalGaussianProcessImuFactor::new(
                    group.measurements,
                    self.config.gyroscope_noise,
                    self.config.accelerometer_noise,
                    self.config.use_accelerometer_measurements,
                    self.config.gravity,
                    group.start_time,
                    group.end_time,
                );
                graph.add_factor(FactorBuilder::new(residual, keys).build());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ImuKnotOrientation {
    index: u32,
    orientation: SO3,
}
