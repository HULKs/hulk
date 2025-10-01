use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use coordinate_systems::Robot;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use hardware::{SensorInterface, TimeInterface};
use linear_algebra::{Orientation3, Vector2, Vector3};
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, joints::Joints, sensor_data::SensorData};

#[derive(Default, Serialize, Deserialize)]
enum State {
    #[default]
    WaitingForSteady,
    CalibratingGravity {
        filtered_gravity: LowPassFilter<Vector3<Robot>>,
        filtered_roll_pitch: LowPassFilter<Vector2<Robot>>,
        remaining_cycles: usize,
    },
    Calibrated {
        calibration: UnitQuaternion<f32>,
    },
}

#[derive(Deserialize, Serialize)]
pub struct SensorDataReceiver {
    last_cycle_start: SystemTime,
    calibration_state: State,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,

    calibration_steady_threshold:
        Parameter<f32, "sensor_data_receiver.calibration_steady_threshold">,
    gravity_calibration_smoothing_factor:
        Parameter<f32, "sensor_data_receiver.gravity_calibration_smoothing_factor">,
    roll_pitch_calibration_smoothing_factor:
        Parameter<f32, "sensor_data_receiver.roll_pitch_calibration_smoothing_factor">,
    number_of_calibration_cycles:
        Parameter<usize, "sensor_data_receiver.number_of_calibration_cycles">,

    maximum_temperature: AdditionalOutput<f32, "maximum_temperature">,
    total_current: AdditionalOutput<f32, "total_current">,

    roll_pitch_calibrated: AdditionalOutput<bool, "roll_pitch_calibrated">,
}

#[context]
pub struct MainOutputs {
    pub sensor_data: MainOutput<SensorData>,
    pub cycle_time: MainOutput<CycleTime>,
}

impl SensorDataReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
            calibration_state: State::WaitingForSteady,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl SensorInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let mut sensor_data = context
            .hardware_interface
            .read_from_sensors()
            .wrap_err("failed to read from sensors")?;

        let measured_angular_velocity = sensor_data.inertial_measurement_unit.angular_velocity;
        let measured_acceleration = sensor_data.inertial_measurement_unit.linear_acceleration;
        let measured_roll_pitch = sensor_data.inertial_measurement_unit.roll_pitch;
        let angular_velocity_sum = measured_angular_velocity.abs().inner.sum();

        let is_steady = angular_velocity_sum < *context.calibration_steady_threshold;
        match &mut self.calibration_state {
            State::WaitingForSteady => {
                if is_steady {
                    self.calibration_state = State::CalibratingGravity {
                        filtered_gravity: LowPassFilter::with_smoothing_factor(
                            measured_acceleration,
                            *context.gravity_calibration_smoothing_factor,
                        ),
                        filtered_roll_pitch: LowPassFilter::with_smoothing_factor(
                            measured_roll_pitch,
                            *context.roll_pitch_calibration_smoothing_factor,
                        ),
                        remaining_cycles: *context.number_of_calibration_cycles,
                    }
                }
            }
            State::CalibratingGravity {
                filtered_gravity,
                filtered_roll_pitch,
                remaining_cycles,
            } => {
                if is_steady {
                    filtered_gravity.update(measured_acceleration);
                    filtered_roll_pitch.update(measured_roll_pitch);
                    *remaining_cycles -= 1;

                    if *remaining_cycles == 0 {
                        let gravity = -filtered_gravity.state().inner;
                        let up = nalgebra::Vector3::y();
                        let orientation = UnitQuaternion::look_at_rh(&gravity, &up);
                        let roll_pitch_orientation = Orientation3::<Robot>::from_euler_angles(
                            -filtered_roll_pitch.state().x(),
                            -filtered_roll_pitch.state().y(),
                            0.0,
                        )
                        .mirror();

                        let roll_pitch_calibration =
                            roll_pitch_orientation.inner.rotation_to(&orientation);

                        self.calibration_state = State::Calibrated {
                            calibration: roll_pitch_calibration,
                        };
                    }
                } else {
                    self.calibration_state = State::WaitingForSteady;
                }
            }
            State::Calibrated { .. } => {}
        }

        if let State::Calibrated { calibration } = self.calibration_state {
            let mut roll_pitch_orientation = Orientation3::<Robot>::from_euler_angles(
                -sensor_data.inertial_measurement_unit.roll_pitch.x(),
                -sensor_data.inertial_measurement_unit.roll_pitch.y(),
                0.0,
            )
            .mirror();

            roll_pitch_orientation.inner = calibration * roll_pitch_orientation.inner;

            let (roll, pitch, _) = roll_pitch_orientation.euler_angles();

            sensor_data.inertial_measurement_unit.roll_pitch.inner.x = roll;
            sensor_data.inertial_measurement_unit.roll_pitch.inner.y = pitch;
        }

        sensor_data.positions = sensor_data.positions - (*context.joint_calibration_offsets);

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };

        context.maximum_temperature.fill_if_subscribed(|| {
            sensor_data
                .temperature_sensors
                .into_iter()
                .fold(0.0, f32::max)
        });

        context
            .total_current
            .fill_if_subscribed(|| sensor_data.currents.into_iter().sum());

        context
            .roll_pitch_calibrated
            .fill_if_subscribed(|| matches!(self.calibration_state, State::Calibrated { .. }));

        self.last_cycle_start = now;
        Ok(MainOutputs {
            sensor_data: sensor_data.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
