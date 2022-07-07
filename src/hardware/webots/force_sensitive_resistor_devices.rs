use anyhow::Context;
use types::{Foot, ForceSensitiveResistors};
use webots::{Robot, TouchSensor};

use super::webots_interface::SIMULATION_TIME_STEP;

pub struct ForceSensitiveResistorDevices {
    left_foot_front_left: TouchSensor,
    left_foot_front_right: TouchSensor,
    left_foot_rear_left: TouchSensor,
    left_foot_rear_right: TouchSensor,
    right_foot_front_left: TouchSensor,
    right_foot_front_right: TouchSensor,
    right_foot_rear_left: TouchSensor,
    right_foot_rear_right: TouchSensor,
}

impl Default for ForceSensitiveResistorDevices {
    fn default() -> Self {
        let left_foot_front_left = Robot::get_touch_sensor("LFoot/FSR/FrontLeft");
        left_foot_front_left.enable(SIMULATION_TIME_STEP);

        let left_foot_front_right = Robot::get_touch_sensor("LFoot/FSR/RearLeft");
        left_foot_front_right.enable(SIMULATION_TIME_STEP);

        let left_foot_rear_left = Robot::get_touch_sensor("LFoot/FSR/FrontRight");
        left_foot_rear_left.enable(SIMULATION_TIME_STEP);

        let left_foot_rear_right = Robot::get_touch_sensor("LFoot/FSR/RearRight");
        left_foot_rear_right.enable(SIMULATION_TIME_STEP);

        let right_foot_front_left = Robot::get_touch_sensor("RFoot/FSR/FrontLeft");
        right_foot_front_left.enable(SIMULATION_TIME_STEP);

        let right_foot_front_right = Robot::get_touch_sensor("RFoot/FSR/RearLeft");
        right_foot_front_right.enable(SIMULATION_TIME_STEP);

        let right_foot_rear_left = Robot::get_touch_sensor("RFoot/FSR/FrontRight");
        right_foot_rear_left.enable(SIMULATION_TIME_STEP);

        let right_foot_rear_right = Robot::get_touch_sensor("RFoot/FSR/RearRight");
        right_foot_rear_right.enable(SIMULATION_TIME_STEP);

        Self {
            left_foot_front_left,
            left_foot_front_right,
            left_foot_rear_left,
            left_foot_rear_right,
            right_foot_front_left,
            right_foot_front_right,
            right_foot_rear_left,
            right_foot_rear_right,
        }
    }
}

impl ForceSensitiveResistorDevices {
    pub fn get_values(&self) -> anyhow::Result<ForceSensitiveResistors> {
        let left_foot_front_left_values = self
            .left_foot_front_left
            .get_values()
            .context("Failed to get front left force sensitive resistor of left foot")?;
        let left_foot_front_right_values = self
            .left_foot_front_right
            .get_values()
            .context("Failed to get front right force sensitive resistor of left foot")?;
        let left_foot_rear_left_values = self
            .left_foot_rear_left
            .get_values()
            .context("Failed to get rear left force sensitive resistor of left foot")?;
        let left_foot_rear_right_values = self
            .left_foot_rear_right
            .get_values()
            .context("Failed to get rear right force sensitive resistor of left foot")?;
        let right_foot_front_left_values = self
            .right_foot_front_left
            .get_values()
            .context("Failed to get front left force sensitive resistor of right foot")?;
        let right_foot_front_right_values = self
            .right_foot_front_right
            .get_values()
            .context("Failed to get front right force sensitive resistor of right foot")?;
        let right_foot_rear_left_values = self
            .right_foot_rear_left
            .get_values()
            .context("Failed to get rear left force sensitive resistor of right foot")?;
        let right_foot_rear_right_values = self
            .right_foot_rear_right
            .get_values()
            .context("Failed to get rear right force sensitive resistor of right foot")?;
        Ok(ForceSensitiveResistors {
            left: Foot {
                front_left: left_foot_front_left_values[2] as f32,
                front_right: left_foot_front_right_values[2] as f32,
                rear_left: left_foot_rear_left_values[2] as f32,
                rear_right: left_foot_rear_right_values[2] as f32,
            },
            right: Foot {
                front_left: right_foot_front_left_values[2] as f32,
                front_right: right_foot_front_right_values[2] as f32,
                rear_left: right_foot_rear_left_values[2] as f32,
                rear_right: right_foot_rear_right_values[2] as f32,
            },
        })
    }
}
