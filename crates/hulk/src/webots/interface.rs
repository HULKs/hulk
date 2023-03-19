use std::{
    str::from_utf8,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::{
    eyre::{bail, eyre, Error, WrapErr},
    Result,
};
use serde::Deserialize;
use spl_network::endpoint::{Endpoint, Ports};
use tokio::{
    runtime::{Builder, Runtime},
    select,
};
use tokio_util::sync::CancellationToken;
use types::{
    hardware::{self, Ids},
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    ycbcr422_image::YCbCr422Image,
    CameraPosition, Joints, Leds, SensorData,
};
use webots::Robot;

use super::{
    camera::Camera, force_sensitive_resistor_devices::ForceSensitiveResistorDevices,
    intertial_measurement_unit_devices::InertialMeasurementUnitDevices,
    joint_devices::JointDevices, keyboard_device::KeyboardDevice,
    sonar_sensor_devices::SonarSensorDevices,
};

pub const SIMULATION_TIME_STEP: i32 = 10;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub spl_network_ports: Ports,
}

pub struct Interface {
    _robot: Robot,

    inertial_measurement_unit: InertialMeasurementUnitDevices,
    sonar_sensors: SonarSensorDevices,
    force_sensitive_resistors: ForceSensitiveResistorDevices,
    joints: JointDevices,
    keyboard: KeyboardDevice,
    top_camera: Camera,
    bottom_camera: Camera,
    top_camera_requested: AtomicBool,
    bottom_camera_requested: AtomicBool,
    spl_network_endpoint: Endpoint,
    async_runtime: Runtime,
    keep_running: CancellationToken,
    simulator_audio_synchronization: Barrier,
}

impl Interface {
    pub fn new(keep_running: CancellationToken, parameters: Parameters) -> Result<Self> {
        let robot = Default::default();
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .wrap_err("failed to create tokio runtime")?;

        Ok(Self {
            _robot: robot,
            inertial_measurement_unit: Default::default(),
            sonar_sensors: Default::default(),
            force_sensitive_resistors: Default::default(),
            joints: Default::default(),
            keyboard: Default::default(),
            top_camera: Camera::new(CameraPosition::Top),
            bottom_camera: Camera::new(CameraPosition::Bottom),
            top_camera_requested: AtomicBool::new(false),
            bottom_camera_requested: AtomicBool::new(false),
            spl_network_endpoint: runtime
                .block_on(Endpoint::new(parameters.spl_network_ports))
                .wrap_err("failed to initialize SPL network")?,
            async_runtime: runtime,
            keep_running,
            simulator_audio_synchronization: Barrier::new(2),
        })
    }

    fn step_simulation(&self) -> Result<()> {
        if Robot::step(SIMULATION_TIME_STEP) == -1 {
            // initiate tear down very early
            self.keep_running.cancel();
            bail!("termination requested");
        }
        Ok(())
    }

    fn update_cameras(&self) -> Result<()> {
        if self
            .top_camera_requested
            .compare_exchange_weak(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.top_camera
                .update_image()
                .wrap_err("failed to update top camera image")?;
        }

        if self
            .bottom_camera_requested
            .compare_exchange_weak(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.bottom_camera
                .update_image()
                .wrap_err("failed to update bottom camera image")?;
        }

        Ok(())
    }
}

impl hardware::Interface for Interface {
    fn read_from_microphones(&self) -> Result<Samples> {
        self.simulator_audio_synchronization.wait();
        if self.keep_running.is_cancelled() {
            bail!("termination requested");
        }
        Ok(Samples {
            rate: 0,
            channels_of_samples: Arc::new(vec![]),
        })
    }

    fn get_now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs_f64(Robot::get_time())
    }

    fn get_ids(&self) -> Ids {
        let name = from_utf8(Robot::get_name()).expect("robot name must be valid UTF-8");
        Ids {
            body_id: name.to_string(),
            head_id: name.to_string(),
        }
    }

    fn read_from_sensors(&self) -> Result<SensorData> {
        match self.step_simulation().wrap_err("failed to step simulation") {
            Ok(_) => {
                self.simulator_audio_synchronization.wait();
            }
            Err(error) => {
                self.simulator_audio_synchronization.wait();
                self.top_camera.unblock_read();
                self.bottom_camera.unblock_read();
                return Err(error);
            }
        };
        if self.keep_running.is_cancelled() {
            bail!("termination requested");
        }
        let positions = self.joints.get_positions();
        let inertial_measurement_unit = self
            .inertial_measurement_unit
            .get_values()
            .wrap_err("failed to get inertial measurement unit values")?;
        let sonar_sensors = self.sonar_sensors.get_values();
        let force_sensitive_resistors = self
            .force_sensitive_resistors
            .get_values()
            .wrap_err("failed to get force sensitive resistor values")?;
        let touch_sensors = self.keyboard.get_touch_sensors();

        self.update_cameras().wrap_err("failed to update cameras")?;

        Ok(SensorData {
            positions,
            inertial_measurement_unit,
            sonar_sensors,
            force_sensitive_resistors,
            touch_sensors,
        })
    }

    fn write_to_actuators(
        &self,
        positions: Joints,
        _stiffnesses: Joints,
        _leds: Leds,
    ) -> Result<()> {
        self.joints
            .head
            .yaw
            .motor
            .set_position(positions.head.yaw.into());
        self.joints
            .head
            .pitch
            .motor
            .set_position(positions.head.pitch.into());
        self.joints
            .left_arm
            .shoulder_pitch
            .motor
            .set_position(positions.left_arm.shoulder_pitch.into());
        self.joints
            .left_arm
            .shoulder_roll
            .motor
            .set_position(positions.left_arm.shoulder_roll.into());
        self.joints
            .left_arm
            .elbow_yaw
            .motor
            .set_position(positions.left_arm.elbow_yaw.into());
        self.joints
            .left_arm
            .elbow_roll
            .motor
            .set_position(positions.left_arm.elbow_roll.into());
        self.joints
            .left_arm
            .wrist_yaw
            .motor
            .set_position(positions.left_arm.wrist_yaw.into());
        self.joints
            .left_leg
            .hip_yaw_pitch
            .motor
            .set_position(positions.left_leg.hip_yaw_pitch.into());
        self.joints
            .left_leg
            .hip_roll
            .motor
            .set_position(positions.left_leg.hip_roll.into());
        self.joints
            .left_leg
            .hip_pitch
            .motor
            .set_position(positions.left_leg.hip_pitch.into());
        self.joints
            .left_leg
            .knee_pitch
            .motor
            .set_position(positions.left_leg.knee_pitch.into());
        self.joints
            .left_leg
            .ankle_pitch
            .motor
            .set_position(positions.left_leg.ankle_pitch.into());
        self.joints
            .left_leg
            .ankle_roll
            .motor
            .set_position(positions.left_leg.ankle_roll.into());
        self.joints
            .right_leg
            .hip_yaw_pitch
            .motor
            .set_position(positions.right_leg.hip_yaw_pitch.into());
        self.joints
            .right_leg
            .hip_roll
            .motor
            .set_position(positions.right_leg.hip_roll.into());
        self.joints
            .right_leg
            .hip_pitch
            .motor
            .set_position(positions.right_leg.hip_pitch.into());
        self.joints
            .right_leg
            .knee_pitch
            .motor
            .set_position(positions.right_leg.knee_pitch.into());
        self.joints
            .right_leg
            .ankle_pitch
            .motor
            .set_position(positions.right_leg.ankle_pitch.into());
        self.joints
            .right_leg
            .ankle_roll
            .motor
            .set_position(positions.right_leg.ankle_roll.into());
        self.joints
            .right_arm
            .shoulder_pitch
            .motor
            .set_position(positions.right_arm.shoulder_pitch.into());
        self.joints
            .right_arm
            .shoulder_roll
            .motor
            .set_position(positions.right_arm.shoulder_roll.into());
        self.joints
            .right_arm
            .elbow_yaw
            .motor
            .set_position(positions.right_arm.elbow_yaw.into());
        self.joints
            .right_arm
            .elbow_roll
            .motor
            .set_position(positions.right_arm.elbow_roll.into());
        self.joints
            .right_arm
            .wrist_yaw
            .motor
            .set_position(positions.right_arm.wrist_yaw.into());
        self.joints
            .left_arm
            .hand
            .motor
            .set_position(positions.left_arm.hand.into());
        self.joints
            .right_arm
            .hand
            .motor
            .set_position(positions.right_arm.hand.into());
        // Webots robot model does not have stiffnesses
        // Webots robot model does not have LEDs
        Ok(())
    }

    fn read_from_network(&self) -> Result<IncomingMessage> {
        self.async_runtime.block_on(async {
            select! {
                result =  self.spl_network_endpoint.read() => {
                    result.map_err(Error::from)
                },
                _ = self.keep_running.cancelled() => {
                    Err(eyre!("termination requested"))
                }
            }
        })
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.async_runtime
            .block_on(self.spl_network_endpoint.write(message));
        Ok(())
    }

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image> {
        let result = match camera_position {
            CameraPosition::Top => {
                self.top_camera_requested.store(true, Ordering::SeqCst);
                self.top_camera
                    .read()
                    .wrap_err("failed to read from top camera")
            }
            CameraPosition::Bottom => {
                self.bottom_camera_requested.store(true, Ordering::SeqCst);
                self.bottom_camera
                    .read()
                    .wrap_err("failed to read from bottom camera")
            }
        };
        if self.keep_running.is_cancelled() {
            bail!("termination requested");
        }
        result
    }
}
