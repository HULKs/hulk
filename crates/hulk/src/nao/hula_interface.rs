use std::{
    os::unix::net::UnixStream,
    str::from_utf8,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use types::{CycleInfo, Joints, Leds, SensorData};

use hardware::HardwareIds;

use super::hula::{read_from_hula, write_to_hula, ControlStorage};

pub struct HulaInterface {
    control_storage: ControlStorage,
    stream: UnixStream,
    last_sensor_data_production: SystemTime,
    ids: HardwareIds,
}

impl HulaInterface {
    pub fn new() -> anyhow::Result<Self> {
        let mut stream = UnixStream::connect("/tmp/hula").context("Failed to open HULA socket")?;
        let state_storage = read_from_hula(&mut stream).context("Failed to read from HULA")?;
        let ids = HardwareIds {
            body_id: from_utf8(&state_storage.robot_configuration.body_id)
                .context("Failed to convert body ID into UTF-8")?
                .to_string(),
            head_id: from_utf8(&state_storage.robot_configuration.head_id)
                .context("Failed to convert head ID into UTF-8")?
                .to_string(),
        };
        Ok(Self {
            control_storage: Default::default(),
            stream,
            last_sensor_data_production: UNIX_EPOCH,
            ids,
        })
    }

    pub fn get_ids(&self) -> HardwareIds {
        self.ids.clone()
    }

    pub fn set_leds(&mut self, leds: Leds) {
        self.control_storage.left_ear = leds.left_ear.into();
        self.control_storage.right_ear = leds.right_ear.into();
        self.control_storage.chest = leds.chest.into();
        self.control_storage.left_foot = leds.left_foot.into();
        self.control_storage.right_foot = leds.right_foot.into();
        self.control_storage.left_eye = leds.left_eye.into();
        self.control_storage.right_eye = leds.right_eye.into();
    }

    pub fn set_joint_positions(&mut self, requested_positions: Joints) {
        self.control_storage.position = requested_positions.into();
    }

    pub fn set_joint_stiffnesses(&mut self, requested_stiffnesses: Joints) {
        self.control_storage.stiffness = requested_stiffnesses.into();
    }

    pub fn produce_sensor_data(&mut self) -> anyhow::Result<SensorData> {
        write_to_hula(&mut self.stream, self.control_storage)?;
        let state_storage = read_from_hula(&mut self.stream)?;
        let cycle_info = self.produce_cycle_info(state_storage.received_at);
        let positions = state_storage.position.into();
        let inertial_measurement_unit = state_storage.inertial_measurement_unit.into();
        let sonar_sensors = state_storage.sonar_sensors.into();
        let force_sensitive_resistors = state_storage.force_sensitive_resistors.into();
        let touch_sensors = state_storage.touch_sensors.into();

        Ok(SensorData {
            cycle_info,
            positions,
            inertial_measurement_unit,
            sonar_sensors,
            force_sensitive_resistors,
            touch_sensors,
        })
    }

    fn produce_cycle_info(&mut self, received_at: f32) -> CycleInfo {
        let now = UNIX_EPOCH + Duration::from_secs_f32(received_at);
        let cycle_info = CycleInfo {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_sensor_data_production)
                .expect("NAO time has run backwards"),
        };
        self.last_sensor_data_production = now;
        cycle_info
    }
}
