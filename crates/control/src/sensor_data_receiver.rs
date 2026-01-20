use std::time::{SystemTime, UNIX_EPOCH};

use booster::{FallDownState, ImuState, MotorState};
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use coordinate_systems::Robot;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use hardware::{FallDownStateInterface, LowStateInterface, TimeInterface};
use linear_algebra::Vector3;
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, joints::Joints};

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
    pub fall_down_state: MainOutput<FallDownState>,
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
        context: CycleContext<impl LowStateInterface + FallDownStateInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let low_state = context
            .hardware_interface
            .read_low_state()
            .wrap_err("failed to read from sensors")?;

        let fall_down_state = context
            .hardware_interface
            .read_fall_down_state()
            .wrap_err("failed to read from sensors")?;

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        // TODO: the simulator sends serial motor states, while
        // the robot sends parallel motor states
        Ok(MainOutputs {
            imu_state: low_state.imu_state.into(),
            serial_motor_states: low_state
                .motor_state_parallel
                .into_iter()
                .collect::<Joints<MotorState>>()
                .into(),
            fall_down_state: fall_down_state.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
