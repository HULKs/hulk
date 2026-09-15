//! Temporary zero-pose body source for central motion's head-controller test path.
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use kinematics::joints::Joints;
use motion_inference::config::{Parameters, Policy};
use ros_z::prelude::*;
use types::robot_command::{JointsCommand, MotorCommand};

pub async fn run(context: Arc<Context>) -> Result<()> {
    let node = context
        .create_node("motion_inference_dummy")
        .build()
        .await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion_inference")?;
    let publisher = node
        .publisher::<JointsCommand>("motion_inference/dummy_joints")
        .build()
        .await?;
    let mut tick = node.create_timer(Duration::from_millis(20));
    loop {
        let parameters = parameters.snapshot();
        let (kp, kd) = Policy::Walk.gains(parameters.typed());
        publisher.publish(&dummy_pose(kp, kd)).await?;
        tick.tick().await;
    }
}

fn dummy_pose(kp: Joints<f32>, kd: Joints<f32>) -> JointsCommand {
    kp.into_iter()
        .zip(kd)
        .map(|(kp, kd)| MotorCommand {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp,
            kd,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bevy_mujoco::{MjcfObject, MujocoBody, MujocoWorld, MujocoWorldPlugin, SimulationMode},
        robot_io::RobotBinding,
    };
    use bevy::prelude::*;

    #[test]
    fn head_yaw_targets_move_the_simulated_robot_and_visual() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(SimulationMode::Paused);
        let robot = app
            .world_mut()
            .spawn((
                MjcfObject::new(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml"),
                    "Trunk",
                )
                .with_free_joint("world_joint")
                .grounded(),
                Transform::default(),
            ))
            .id();
        let head = app
            .world_mut()
            .spawn((MujocoBody::new(robot, "Head_1"), Transform::default()))
            .id();
        app.update();
        let prefix = format!("object_{}_", robot.to_bits());
        let binding =
            RobotBinding::new(app.world().resource::<MujocoWorld>().data(), &prefix).unwrap();
        let mut kp: Joints<f32> = [80.0; 22].into_iter().collect();
        let mut kd: Joints<f32> = [4.0; 22].into_iter().collect();
        kp.head.yaw = 10.0;
        kd.head.yaw = 1.2;
        let mut head_rotations = Vec::new();
        for (time, expected_yaw) in [(1.0, 0.5), (3.0, -0.5)] {
            {
                let mut world = app.world_mut().resource_mut::<MujocoWorld>();
                let data = world.data_mut();
                while data.time() < time {
                    let mut pose = dummy_pose(kp, kd);
                    pose.head.yaw.position = expected_yaw;
                    let command = booster::LowCommand {
                        command_type: booster::CommandType::Serial,
                        motor_commands: pose
                            .into_iter()
                            .map(|motor| booster::MotorCommand {
                                position: motor.position,
                                velocity: motor.velocity,
                                torque: motor.torque,
                                kp: motor.kp,
                                kd: motor.kd,
                                weight: 1.0,
                                command_type: booster::CommandType::Serial,
                            })
                            .collect(),
                    };
                    binding.apply(data, Some(&command));
                    data.step();
                }
                data.forward();
                let yaw = binding.observe(data).low_state.motor_state_serial[0].position;
                assert!(
                    (yaw - expected_yaw).abs() < 0.1,
                    "head yaw at {time}s: expected {expected_yaw}, got {yaw}"
                );
            }
            app.update();
            head_rotations.push(app.world().get::<Transform>(head).unwrap().rotation);
        }
        assert!(head_rotations[0].angle_between(head_rotations[1]) > 0.5);
    }

    #[test]
    fn zero_pose_preserves_configured_gains() {
        let kp: Joints<f32> = (0..22).map(|i| 10.0 + i as f32).collect();
        let kd: Joints<f32> = (0..22).map(|i| 0.5 + i as f32 * 0.1).collect();
        for (i, motor) in dummy_pose(kp, kd).into_iter().enumerate() {
            assert_eq!(motor.position, 0.0);
            assert_eq!(motor.velocity, 0.0);
            assert_eq!(motor.torque, 0.0);
            assert_eq!(motor.kp, 10.0 + i as f32);
            assert_eq!(motor.kd, 0.5 + i as f32 * 0.1);
        }
    }
}
