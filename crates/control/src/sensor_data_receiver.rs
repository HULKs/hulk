use std::{
    f32::consts::FRAC_PI_2,
    time::{SystemTime, UNIX_EPOCH},
};

use booster::{ImuState, LowState, MotorState};
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use coordinate_systems::{Camera, Robot, World};
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use hardware::{LowStateInterface, TimeInterface};
use linear_algebra::{IntoTransform, Isometry3, Vector3};
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
    sensor_data::SensorData,
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
}

#[context]
pub struct MainOutputs {
    pub imu_state: MainOutput<ImuState>,
    pub serial_motor_states: MainOutput<Joints<MotorState>>,
    pub cycle_time: MainOutput<CycleTime>,
    pub sensor_data: MainOutput<SensorData>,
    pub low_state: MainOutput<LowState>,
    pub camera_to_world: MainOutput<Isometry3<Camera, World>>,
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
        context: CycleContext<impl LowStateInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let low_state = context
            .hardware_interface
            .read_low_state()
            .wrap_err("failed to read from sensors")?;

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        let positions = Joints {
            head: HeadJoints {
                yaw: low_state.motor_state_serial[0].position,
                pitch: low_state.motor_state_serial[1].position,
            },
            left_arm: ArmJoints {
                shoulder_pitch: low_state.motor_state_serial[2].position,
                shoulder_roll: low_state.motor_state_serial[3].position,
                shoulder_yaw: low_state.motor_state_serial[4].position,
                elbow: low_state.motor_state_serial[5].position,
            },
            right_arm: ArmJoints {
                shoulder_pitch: low_state.motor_state_serial[6].position,
                shoulder_roll: low_state.motor_state_serial[7].position,
                shoulder_yaw: low_state.motor_state_serial[8].position,
                elbow: low_state.motor_state_serial[9].position,
            },
            left_leg: LegJoints {
                hip_pitch: low_state.motor_state_serial[10].position,
                hip_roll: low_state.motor_state_serial[11].position,
                hip_yaw: low_state.motor_state_serial[12].position,
                knee: low_state.motor_state_serial[13].position,
                ankle_up: low_state.motor_state_serial[14].position,
                ankle_down: low_state.motor_state_serial[15].position,
            },
            right_leg: LegJoints {
                hip_pitch: low_state.motor_state_serial[16].position,
                hip_roll: low_state.motor_state_serial[17].position,
                hip_yaw: low_state.motor_state_serial[18].position,
                knee: low_state.motor_state_serial[19].position,
                ankle_up: low_state.motor_state_serial[20].position,
                ankle_down: low_state.motor_state_serial[21].position,
            },
        };

        let sensor_data = SensorData {
            positions,
            ..SensorData::default()
        };

        let raw = low_state.camera_to_world;

        let rotation = nalgebra::Rotation3::from_matrix_unchecked(
            nalgebra::Matrix3::from_row_slice(&raw[3..]),
        );
        let translation =
            nalgebra::Translation3::from(nalgebra::Vector3::from_row_slice(&raw[..3]));
        let camera_to_world = nalgebra::Isometry3::from_parts(translation, rotation.into())
            * UnitQuaternion::from_euler_angles(-FRAC_PI_2, FRAC_PI_2, 0.0);

        Ok(MainOutputs {
            imu_state: low_state.imu_state.into(),
            serial_motor_states: low_state
                .motor_state_serial
                .iter()
                .cloned()
                .collect::<Joints<MotorState>>()
                .into(),
            cycle_time: cycle_time.into(),
            sensor_data: sensor_data.into(),
            low_state: low_state.into(),
            camera_to_world: camera_to_world.framed_transform().into(),
        })
    }
}
