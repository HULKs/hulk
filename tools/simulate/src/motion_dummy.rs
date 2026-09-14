//! Temporary head-yaw sine command source. Replace this launcher entry with `motion` after rebasing.
use std::{f32::consts::TAU, sync::Arc, time::Duration};

use color_eyre::Result;
use kinematics::joints::Joints;
use motion_inference::config::{Parameters, Policy};
use ros_z::prelude::*;
use types::robot_command::{JointsCommand, MotionCommand, MotionType, MotorCommand};

const HEAD_YAW_AMPLITUDE: f32 = 0.5;
const HEAD_YAW_PERIOD: f32 = 4.0;

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
    let start = node.clock().now();
    loop {
        let parameters = parameters.snapshot();
        let (kp, kd) = Policy::Walk.gains(parameters.typed());
        let elapsed = node.clock().now().duration_since(start);
        publisher.publish(&dummy_pose(kp, kd, elapsed)).await?;
        tick.tick().await;
    }
}

fn dummy_pose(kp: Joints<f32>, kd: Joints<f32>, elapsed: Duration) -> MotionCommand {
    let mut joints_command: JointsCommand = kp
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
    let angular_frequency = TAU / HEAD_YAW_PERIOD;
    let phase = angular_frequency * (elapsed.as_secs_f64() % f64::from(HEAD_YAW_PERIOD)) as f32;
    joints_command.head.yaw.position = HEAD_YAW_AMPLITUDE * phase.sin();
    joints_command.head.yaw.velocity = HEAD_YAW_AMPLITUDE * angular_frequency * phase.cos();
    MotionCommand {
        // This variant selects Booster Custom mode for full serial joint control.
        motion_type: MotionType::Walk,
        joints_command,
    }
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
    fn head_yaw_sine_moves_the_simulated_robot_and_visual() {
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
                    let pose = dummy_pose(kp, kd, Duration::from_secs_f64(data.time()));
                    let command = booster::LowCommand {
                        command_type: booster::CommandType::Serial,
                        motor_commands: pose
                            .joints_command
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
    fn head_yaw_oscillates_while_other_joints_and_gains_stay_fixed() {
        let kp: Joints<f32> = (0..22).map(|i| 10.0 + i as f32).collect();
        let kd: Joints<f32> = (0..22).map(|i| 0.5 + i as f32 * 0.1).collect();
        for (seconds, yaw, yaw_velocity) in [
            (0, 0.0, std::f32::consts::FRAC_PI_4),
            (1, 0.5, 0.0),
            (2, 0.0, -std::f32::consts::FRAC_PI_4),
            (3, -0.5, 0.0),
            (4, 0.0, std::f32::consts::FRAC_PI_4),
        ] {
            let command = dummy_pose(kp, kd, Duration::from_secs(seconds));
            assert!((command.joints_command.head.yaw.position - yaw).abs() < 1e-6);
            assert!((command.joints_command.head.yaw.velocity - yaw_velocity).abs() < 1e-6);
            for (i, motor) in command.joints_command.into_iter().enumerate() {
                if i != 0 {
                    assert_eq!(motor.position, 0.0);
                    assert_eq!(motor.velocity, 0.0);
                }
                assert_eq!(motor.torque, 0.0);
                assert_eq!(motor.kp, 10.0 + i as f32);
                assert_eq!(motor.kd, 0.5 + i as f32 * 0.1);
            }
        }
    }
}
