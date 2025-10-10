use std::time::{SystemTime, UNIX_EPOCH};

use booster_low_level_interface::SimulationMessage;
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use coordinate_systems::Robot;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use hardware::{LowStateInterface, TimeInterface};
use linear_algebra::Vector3;
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::Joints,
    sensor_data::{InertialMeasurementUnitData, SensorData},
};

#[derive(Default, Serialize, Deserialize)]
enum State {
    #[default]
    WaitingForSteady,
    CalibratingGravity {
        filtered_gravity: LowPassFilter<Vector3<Robot>>,
        filtered_roll_pitch_yaw: LowPassFilter<Vector3<Robot>>,
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
        mut context: CycleContext<impl LowStateInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let SimulationMessage { time, payload } = context
            .hardware_interface
            .read_low_state()
            .wrap_err("failed to read from sensors")?;

        let low_state = payload;

        // let measured_angular_velocity = low_state.imu_state.angular_velocity;
        // let measured_acceleration = low_state.imu_state.linear_acceleration;
        // let measured_roll_pitch_yaw = low_state.imu_state.roll_pitch_yaw;

        // match &mut self.calibration_state {
        //     State::WaitingForSteady => {
        //         if measured_angular_velocity.abs().inner.sum()
        //             < *context.calibration_steady_threshold
        //         {
        //             self.calibration_state = State::CalibratingGravity {
        //                 filtered_gravity: LowPassFilter::with_smoothing_factor(
        //                     measured_acceleration,
        //                     *context.gravity_calibration_smoothing_factor,
        //                 ),
        //                 filtered_roll_pitch_yaw: LowPassFilter::with_smoothing_factor(
        //                     measured_roll_pitch_yaw,
        //                     *context.roll_pitch_calibration_smoothing_factor,
        //                 ),
        //                 remaining_cycles: *context.number_of_calibration_cycles,
        //             }
        //         }
        //     }
        //     State::CalibratingGravity {
        //         filtered_gravity,
        //         filtered_roll_pitch_yaw: filtered_roll_pitch,
        //         remaining_cycles,
        //     } => {
        //         if measured_angular_velocity.abs().inner.sum()
        //             < *context.calibration_steady_threshold
        //         {
        //             filtered_gravity.update(measured_acceleration);
        //             filtered_roll_pitch.update(measured_roll_pitch_yaw);
        //             *remaining_cycles -= 1;

        //             if *remaining_cycles == 0 {
        //                 let gravity = -filtered_gravity.state().inner;
        //                 let up = nalgebra::Vector3::y();
        //                 let orientation = UnitQuaternion::look_at_rh(&gravity, &up);
        //                 let roll_pitch_orientation = Orientation3::<Robot>::from_euler_angles(
        //                     -filtered_roll_pitch.state().x(),
        //                     -filtered_roll_pitch.state().y(),
        //                     0.0,
        //                 )
        //                 .mirror();

        //                 let roll_pitch_calibration =
        //                     roll_pitch_orientation.inner.rotation_to(&orientation);

        //                 self.calibration_state = State::Calibrated {
        //                     calibration: roll_pitch_calibration,
        //                 };
        //             }
        //         } else {
        //             self.calibration_state = State::WaitingForSteady;
        //         }
        //     }
        //     State::Calibrated { .. } => {}
        // }

        // if let State::Calibrated { calibration } = self.calibration_state {
        //     let mut roll_pitch_orientation = Orientation3::<Robot>::from_euler_angles(
        //         -low_state.imu_state.roll_pitch_yaw.x(),
        //         -low_state.imu_state.roll_pitch_yaw.y(),
        //         0.0,
        //     )
        //     .mirror();

        //     roll_pitch_orientation.inner = calibration * roll_pitch_orientation.inner;

        //     let (roll, pitch, _) = roll_pitch_orientation.euler_angles();

        //     low_state.imu_state.roll_pitch_yaw.inner.x = roll;
        //     low_state.imu_state.roll_pitch_yaw.inner.y = pitch;
        //     low_state.imu_state.roll_pitch_yaw.inner.z = pitch;
        // }

        // sensor_data.positions = sensor_data.positions - (*context.joint_calibration_offsets);

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };

        // Faked data
        context.maximum_temperature.fill_if_subscribed(|| 42.);

        // Faked data
        context.total_current.fill_if_subscribed(|| 42.);

        context
            .roll_pitch_calibrated
            .fill_if_subscribed(|| matches!(self.calibration_state, State::Calibrated { .. }));

        let half_fake_senor_data = SensorData {
            positions: (&low_state).into(),
            inertial_measurement_unit: InertialMeasurementUnitData {
                linear_acceleration: low_state.imu_state.linear_acceleration,
                angular_velocity: low_state.imu_state.angular_velocity,
                roll_pitch: low_state.imu_state.roll_pitch_yaw.clone().xy(),
            },
            ..Default::default()
        };

        self.last_cycle_start = now;

        Ok(MainOutputs {
            sensor_data: half_fake_senor_data.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
