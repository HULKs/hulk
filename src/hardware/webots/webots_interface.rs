use std::{
    str::from_utf8,
    sync::{
        atomic::{AtomicBool, Ordering},
        Barrier,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context};
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;
use types::{CameraPosition, CycleInfo, Image422, Joints, Leds, SensorData};
use webots::Robot;

use crate::hardware::{
    interface::{HardwareIds, NUMBER_OF_AUDIO_CHANNELS, NUMBER_OF_AUDIO_SAMPLES},
    HardwareInterface,
};

use super::{
    camera::Camera, force_sensitive_resistor_devices::ForceSensitiveResistorDevices,
    intertial_measurement_unit_devices::InertialMeasurementUnitDevices,
    joint_devices::JointDevices, keyboard_device::KeyboardDevice,
    sonar_sensor_devices::SonarSensorDevices,
};

pub const SIMULATION_TIME_STEP: i32 = 10;

pub struct WebotsInterface {
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

    last_sensor_data_production: Mutex<SystemTime>,
    keep_running: CancellationToken,

    control_audio_synchronization: Barrier,
    audio_buffer: Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]>,
}

impl WebotsInterface {
    pub fn new(keep_running: CancellationToken) -> Self {
        let robot = Default::default();

        Self {
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

            last_sensor_data_production: Mutex::new(UNIX_EPOCH),
            keep_running,

            control_audio_synchronization: Barrier::new(2),
            audio_buffer: Mutex::new([[0.0; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]),
        }
    }

    fn step_simulation(&self) -> anyhow::Result<()> {
        if Robot::step(SIMULATION_TIME_STEP) == -1 {
            // initiate tear down very early
            self.keep_running.cancel();
            bail!("Termination requested");
        }
        Ok(())
    }

    fn update_cameras(&self) -> anyhow::Result<()> {
        if self
            .top_camera_requested
            .compare_exchange_weak(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.top_camera
                .update_image()
                .context("Failed to update top camera image")?;
        }

        if self
            .bottom_camera_requested
            .compare_exchange_weak(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.bottom_camera
                .update_image()
                .context("Failed to update bottom camera image")?;
        }

        Ok(())
    }

    fn produce_cycle_info(&self) -> CycleInfo {
        let now = UNIX_EPOCH + Duration::from_secs_f64(Robot::get_time());
        let mut last_sensor_data_production = self.last_sensor_data_production.lock();
        let cycle_info = CycleInfo {
            start_time: now,
            last_cycle_duration: now
                .duration_since(*last_sensor_data_production)
                .expect("Webots time has run backwards"),
        };
        *last_sensor_data_production = now;
        cycle_info
    }
}

impl HardwareInterface for WebotsInterface {
    fn get_ids(&self) -> HardwareIds {
        let name = from_utf8(Robot::get_name()).expect("Robot name must be valid UTF-8");
        HardwareIds {
            body_id: name.to_string(),
            head_id: name.to_string(),
        }
    }

    fn set_leds(&self, _leds: Leds) {
        // Webots robot model does not have LEDs
    }

    fn set_joint_positions(&self, requested_positions: Joints) {
        self.joints
            .head
            .yaw
            .motor
            .set_position(requested_positions.head.yaw.into());
        self.joints
            .head
            .pitch
            .motor
            .set_position(requested_positions.head.pitch.into());
        self.joints
            .left_arm
            .shoulder_pitch
            .motor
            .set_position(requested_positions.left_arm.shoulder_pitch.into());
        self.joints
            .left_arm
            .shoulder_roll
            .motor
            .set_position(requested_positions.left_arm.shoulder_roll.into());
        self.joints
            .left_arm
            .elbow_yaw
            .motor
            .set_position(requested_positions.left_arm.elbow_yaw.into());
        self.joints
            .left_arm
            .elbow_roll
            .motor
            .set_position(requested_positions.left_arm.elbow_roll.into());
        self.joints
            .left_arm
            .wrist_yaw
            .motor
            .set_position(requested_positions.left_arm.wrist_yaw.into());
        self.joints
            .left_leg
            .hip_yaw_pitch
            .motor
            .set_position(requested_positions.left_leg.hip_yaw_pitch.into());
        self.joints
            .left_leg
            .hip_roll
            .motor
            .set_position(requested_positions.left_leg.hip_roll.into());
        self.joints
            .left_leg
            .hip_pitch
            .motor
            .set_position(requested_positions.left_leg.hip_pitch.into());
        self.joints
            .left_leg
            .knee_pitch
            .motor
            .set_position(requested_positions.left_leg.knee_pitch.into());
        self.joints
            .left_leg
            .ankle_pitch
            .motor
            .set_position(requested_positions.left_leg.ankle_pitch.into());
        self.joints
            .left_leg
            .ankle_roll
            .motor
            .set_position(requested_positions.left_leg.ankle_roll.into());
        self.joints
            .right_leg
            .hip_yaw_pitch
            .motor
            .set_position(requested_positions.right_leg.hip_yaw_pitch.into());
        self.joints
            .right_leg
            .hip_roll
            .motor
            .set_position(requested_positions.right_leg.hip_roll.into());
        self.joints
            .right_leg
            .hip_pitch
            .motor
            .set_position(requested_positions.right_leg.hip_pitch.into());
        self.joints
            .right_leg
            .knee_pitch
            .motor
            .set_position(requested_positions.right_leg.knee_pitch.into());
        self.joints
            .right_leg
            .ankle_pitch
            .motor
            .set_position(requested_positions.right_leg.ankle_pitch.into());
        self.joints
            .right_leg
            .ankle_roll
            .motor
            .set_position(requested_positions.right_leg.ankle_roll.into());
        self.joints
            .right_arm
            .shoulder_pitch
            .motor
            .set_position(requested_positions.right_arm.shoulder_pitch.into());
        self.joints
            .right_arm
            .shoulder_roll
            .motor
            .set_position(requested_positions.right_arm.shoulder_roll.into());
        self.joints
            .right_arm
            .elbow_yaw
            .motor
            .set_position(requested_positions.right_arm.elbow_yaw.into());
        self.joints
            .right_arm
            .elbow_roll
            .motor
            .set_position(requested_positions.right_arm.elbow_roll.into());
        self.joints
            .right_arm
            .wrist_yaw
            .motor
            .set_position(requested_positions.right_arm.wrist_yaw.into());
        self.joints
            .left_arm
            .hand
            .motor
            .set_position(requested_positions.left_arm.hand.into());
        self.joints
            .right_arm
            .hand
            .motor
            .set_position(requested_positions.right_arm.hand.into());
    }

    fn set_joint_stiffnesses(&self, _requested_stiffnesses: Joints) {
        // Webots robot model does not have stiffnesses
    }

    fn produce_sensor_data(&self) -> anyhow::Result<SensorData> {
        match self.step_simulation().context("Failed to step simulation") {
            Ok(_) => {
                self.control_audio_synchronization.wait();
            }
            Err(error) => {
                self.control_audio_synchronization.wait();
                self.top_camera.unblock_produce();
                self.bottom_camera.unblock_produce();
                return Err(error);
            }
        };
        let cycle_info = self.produce_cycle_info();
        let positions = self.joints.get_positions();
        let inertial_measurement_unit = self
            .inertial_measurement_unit
            .get_values()
            .context("Failed to get inertial measurement unit values")?;
        let sonar_sensors = self.sonar_sensors.get_values();
        let force_sensitive_resistors = self
            .force_sensitive_resistors
            .get_values()
            .context("Failed to get force sensitive resistor values")?;
        let touch_sensors = self.keyboard.get_touch_sensors();

        self.update_cameras().context("Failed to update cameras")?;

        Ok(SensorData {
            cycle_info,
            positions,
            inertial_measurement_unit,
            sonar_sensors,
            force_sensitive_resistors,
            touch_sensors,
        })
    }

    fn produce_image_data(&self, camera_position: CameraPosition) -> anyhow::Result<CycleInfo> {
        match camera_position {
            CameraPosition::Top => {
                self.top_camera_requested.store(true, Ordering::SeqCst);
                self.top_camera.produce()
            }
            CameraPosition::Bottom => {
                self.bottom_camera_requested.store(true, Ordering::SeqCst);
                self.bottom_camera.produce()
            }
        }
        // TODO generate the correct timing
        let cycle_info = CycleInfo {
            start_time: SystemTime::now(),
            last_cycle_duration: Duration::from_millis(30),
        };
        Ok(cycle_info)
    }

    fn get_image(&self, camera_position: CameraPosition) -> &Mutex<Image422> {
        match camera_position {
            CameraPosition::Top => self.top_camera.get_image(),
            CameraPosition::Bottom => self.bottom_camera.get_image(),
        }
    }

    fn start_image_capture(&self, _camera_position: CameraPosition) -> anyhow::Result<()> {
        Ok(())
    }

    fn produce_audio_data(&self) -> anyhow::Result<()> {
        self.control_audio_synchronization.wait();
        Ok(())
    }

    fn get_audio_buffer(
        &self,
    ) -> &Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]> {
        &self.audio_buffer
    }
}
