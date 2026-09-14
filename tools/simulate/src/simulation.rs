use std::time::Duration;

use bevy::prelude::*;
use ros_z::time::Time as RosTime;

use crate::{
    bevy_mujoco::{MujocoModelUpdateSet, MujocoStepSet, MujocoWorld, SimulationMode},
    robot_io::RobotBinding,
    robotics::Robotics,
    scene::{robot, visual::ObjectVisualAssets},
};

#[derive(Component)]
pub struct ControlledRobot;

#[derive(Default, Resource)]
pub struct SimulationControl {
    pub reset: bool,
}

#[derive(Default, Resource)]
struct Binding {
    generation: Option<u64>,
    robot: Option<RobotBinding>,
    last_input: Option<f64>,
}

pub struct MotionSimulationPlugin;
impl Plugin for MotionSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Binding>()
            .init_resource::<SimulationControl>()
            .insert_resource(SimulationMode::Paused)
            .add_systems(Startup, spawn_robot)
            .add_systems(
                PreUpdate,
                (bind_robot, reset_robot)
                    .chain()
                    .after(MujocoModelUpdateSet),
            )
            .add_systems(FixedUpdate, apply_command.before(MujocoStepSet))
            .add_systems(FixedUpdate, publish_observation.after(MujocoStepSet));
    }
}

fn spawn_robot(mut commands: Commands, assets: Res<ObjectVisualAssets>) {
    let robot = robot::spawn(&mut commands, &assets.robot, Transform::default());
    commands.entity(robot).insert(ControlledRobot);
}

fn bind_robot(
    world: Res<MujocoWorld>,
    robot: Single<Entity, With<ControlledRobot>>,
    mut binding: ResMut<Binding>,
    io: Res<Robotics>,
) {
    if binding.generation == Some(world.generation) || !world.contains_object(*robot) {
        return;
    }
    let robot = RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))
        .expect("controlled K1 must contain the configured joints and sensors");
    // Includes the initial observation while paused; recompilation can relocate all MuJoCo indices.
    io.publish_observation(
        robot.observe(world.data()),
        simulation_time(world.data().time()),
    )
    .expect("publish initial robot observation");
    io.publish_inputs().expect("publish initial UI inputs");
    binding.robot = Some(robot);
    binding.generation = Some(world.generation);
}

fn apply_command(
    mut world: ResMut<MujocoWorld>,
    binding: Res<Binding>,
    io: Res<Robotics>,
    mode: Res<SimulationMode>,
) {
    if *mode == SimulationMode::Paused {
        return;
    }
    if let Some(robot) = &binding.robot {
        robot.apply(world.data_mut(), io.latest_command().as_ref());
    }
}

fn publish_observation(
    mut world: ResMut<MujocoWorld>,
    mut binding: ResMut<Binding>,
    io: Res<Robotics>,
    mode: Res<SimulationMode>,
) {
    if *mode == SimulationMode::Paused {
        return;
    }
    let Some(robot) = &binding.robot else {
        return;
    };
    world.data_mut().forward();
    let time = world.data().time();
    io.publish_observation(robot.observe(world.data()), simulation_time(time))
        .expect("publish robot observation");
    // Repeat the selected inputs for subscribers that start after the UI publishers.
    if binding
        .last_input
        .is_none_or(|previous| time - previous >= 0.02 - 1e-9)
    {
        io.publish_inputs().expect("publish UI inputs");
        binding.last_input = Some(time);
    }
}

fn reset_robot(
    mut world: ResMut<MujocoWorld>,
    robot: Single<Entity, With<ControlledRobot>>,
    mut binding: ResMut<Binding>,
    mut io: ResMut<Robotics>,
    mut control: ResMut<SimulationControl>,
    mut mode: ResMut<SimulationMode>,
    assets: Res<ObjectVisualAssets>,
) {
    if !control.reset {
        return;
    }
    control.reset = false;
    *mode = SimulationMode::Paused;
    let Some(robot_binding) = &binding.robot else {
        return;
    };
    robot_binding.reset_joints(world.data_mut());
    world
        .set_object_pose(
            *robot,
            Transform::from_xyz(0.0, assets.robot.ground_offset(), 0.0),
        )
        .expect("reset robot pose");
    // Preserve monotonic MuJoCo time; restart nodes to clear controller histories and cached commands.
    io.restart().expect("restart motion stack");
    io.publish_observation(
        robot_binding.observe(world.data()),
        simulation_time(world.data().time()),
    )
    .expect("publish reset observation");
    binding.last_input = None;
}

fn simulation_time(seconds: f64) -> RosTime {
    RosTime::from_nanos(Duration::from_secs_f64(seconds).as_nanos() as i64)
}
