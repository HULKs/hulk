use std::{
    os::unix::net::UnixStream,
    str::from_utf8,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::{eyre::WrapErr, Result};
use parking_lot::{Mutex, RwLock};
use types::{hardware::Ids, joints::Joints, led::Leds, sensor_data::SensorData};

use super::{
    double_buffered_reader::{DoubleBufferedReader, SelectPoller},
    hula::{read_from_hula, write_to_hula, ControlStorage, StateStorage},
};

pub const HULA_SOCKET_PATH: &str = "/tmp/hula";

pub struct HulaWrapper {
    now: RwLock<SystemTime>,
    ids: Ids,
    stream: Mutex<UnixStream>,
    hula_reader: Mutex<DoubleBufferedReader<StateStorage, UnixStream, SelectPoller>>,
}

impl HulaWrapper {
    pub fn new() -> Result<Self> {
        let stream =
            UnixStream::connect(HULA_SOCKET_PATH).wrap_err("failed to open HULA socket")?;
        stream
            .set_nonblocking(true)
            .wrap_err("failed to set HULA socket to non-blocking mode")?;
        let mut hula_reader = DoubleBufferedReader::from_reader_and_poller(
            stream
                .try_clone()
                .wrap_err("failed to clone HULA socket for reading")?,
            SelectPoller,
        );
        let state_storage =
            read_from_hula(&mut hula_reader).wrap_err("failed to read from HULA")?;
        let ids = Ids {
            body_id: from_utf8(&state_storage.robot_configuration.body_id)
                .wrap_err("failed to convert body ID into UTF-8")?
                .to_string(),
            head_id: from_utf8(&state_storage.robot_configuration.head_id)
                .wrap_err("failed to convert head ID into UTF-8")?
                .to_string(),
        };
        Ok(Self {
            now: RwLock::new(UNIX_EPOCH),
            ids,
            stream: Mutex::new(stream),
            hula_reader: Mutex::new(hula_reader),
        })
    }

    pub fn get_now(&self) -> SystemTime {
        *self.now.read()
    }

    pub fn get_ids(&self) -> Ids {
        self.ids.clone()
    }

    pub fn read_from_hula(&self) -> Result<SensorData> {
        let state_storage = {
            let mut hula_reader = self.hula_reader.lock();
            read_from_hula(&mut hula_reader).wrap_err("failed to read from HULA")?
        };

        {
            let mut now = self.now.write();
            *now = UNIX_EPOCH + Duration::from_secs_f32(state_storage.received_at);
        }

        let positions = state_storage.position.into();
        let inertial_measurement_unit = state_storage.inertial_measurement_unit.into();
        let sonar_sensors = state_storage.sonar_sensors.into();
        let force_sensitive_resistors = state_storage.force_sensitive_resistors.into();
        let touch_sensors = state_storage.touch_sensors.into();
        let temperature_sensors = state_storage.temperature.into();
        let currents = state_storage.currents.into();

        Ok(SensorData {
            positions,
            inertial_measurement_unit,
            sonar_sensors,
            force_sensitive_resistors,
            touch_sensors,
            temperature_sensors,
            currents,
        })
    }

    pub fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()> {
        let control_storage = ControlStorage {
            left_eye: leds.left_eye.into(),
            right_eye: leds.right_eye.into(),
            chest: leds.chest.into(),
            left_foot: leds.left_foot.into(),
            right_foot: leds.right_foot.into(),
            left_ear: leds.left_ear.into(),
            right_ear: leds.right_ear.into(),
            position: positions.into(),
            stiffness: stiffnesses.into(),
        };

        let mut stream = self.stream.lock();
        write_to_hula(&mut stream, control_storage).wrap_err("failed to write to HULA")
    }
}
