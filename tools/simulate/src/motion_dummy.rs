//! Nominal arm pose and zero-leg fallback for the simulator coordinator.
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
        let locomotion = &parameters.typed().locomotion;
        publisher
            .publish(&dummy_pose(
                kp,
                kd,
                locomotion.shoulder_roll_degrees,
                locomotion.elbow_degrees,
            ))
            .await?;
        tick.tick().await;
    }
}

fn dummy_pose(
    kp: Joints<f32>,
    kd: Joints<f32>,
    shoulder_roll_degrees: f32,
    elbow_degrees: f32,
) -> JointsCommand {
    let mut pose: JointsCommand = kp
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
    for (arm, sign) in [(&mut pose.left_arm, 1.0), (&mut pose.right_arm, -1.0)] {
        arm.shoulder_roll.position = sign * shoulder_roll_degrees.to_radians();
        arm.elbow.position = sign * elbow_degrees.to_radians();
    }
    pose
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
                    let mut pose = dummy_pose(kp, kd, -78.0, -30.0);
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
    fn nominal_pose_uses_arm_parameters_and_preserves_configured_gains() {
        let kp: Joints<f32> = (0..22).map(|i| 10.0 + i as f32).collect();
        let kd: Joints<f32> = (0..22).map(|i| 0.5 + i as f32 * 0.1).collect();
        let pose = dummy_pose(kp, kd, -70.0, -25.0);
        assert_eq!(pose.left_arm.shoulder_roll.position, -70.0_f32.to_radians());
        assert_eq!(pose.right_arm.shoulder_roll.position, 70.0_f32.to_radians());
        assert_eq!(pose.left_arm.elbow.position, -25.0_f32.to_radians());
        assert_eq!(pose.right_arm.elbow.position, 25.0_f32.to_radians());
        for (i, motor) in pose.into_iter().enumerate() {
            if ![3, 5, 7, 9].contains(&i) {
                assert_eq!(motor.position, 0.0);
            }
            assert_eq!(motor.velocity, 0.0);
            assert_eq!(motor.torque, 0.0);
            assert_eq!(motor.kp, 10.0 + i as f32);
            assert_eq!(motor.kd, 0.5 + i as f32 * 0.1);
        }
    }
}
