//! Temporary zero-pose command source. Replace this launcher entry with `motion` after rebasing.
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use kinematics::joints::Joints;
use motion_inference::config::{Parameters, Policy};
use ros_z::prelude::*;
use types::robot_command::{JointsCommand, MotionCommand, MotionType, MotorCommand};

pub async fn run(context: Arc<Context>) -> Result<()> {
    let node = context
        .create_node("motion_inference_dummy")
        .build()
        .await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion_inference")?;
    let publisher = node
        .publisher::<MotionCommand>("commands/motion_command")
        .build()
        .await?;
    let mut tick = node.create_timer(Duration::from_millis(20));
    loop {
        let parameters = parameters.snapshot();
        let (kp, kd) = Policy::Walk.gains(parameters.typed());
        publisher.publish(&zero_pose(kp, kd)).await?;
        tick.tick().await;
    }
}

fn zero_pose(kp: Joints<f32>, kd: Joints<f32>) -> MotionCommand {
    let joints_command: JointsCommand = kp
        .into_iter()
        .zip(kd)
        .map(|(kp, kd)| MotorCommand {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp,
            kd,
        })
        .collect();
    MotionCommand {
        // This variant selects Booster Custom mode for full serial joint control.
        motion_type: MotionType::Walk,
        joints_command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_pose_preserves_each_joints_configured_gains() {
        let kp: Joints<f32> = (0..22).map(|i| 10.0 + i as f32).collect();
        let kd: Joints<f32> = (0..22).map(|i| 0.5 + i as f32 * 0.1).collect();
        for (i, motor) in zero_pose(kp, kd).joints_command.into_iter().enumerate() {
            assert_eq!(motor.position, 0.0);
            assert_eq!(motor.velocity, 0.0);
            assert_eq!(motor.torque, 0.0);
            assert_eq!(motor.kp, 10.0 + i as f32);
            assert_eq!(motor.kd, 0.5 + i as f32 * 0.1);
        }
    }
}
