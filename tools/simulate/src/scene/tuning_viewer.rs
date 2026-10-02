//! Read-only 3D view of the optimizer's original robot sensor and reference topics.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bevy::{
    asset::AssetPlugin, camera::visibility::RenderLayers,
    camera_controller::free_camera::FreeCameraPlugin, prelude::*,
};
use booster::MotorState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground, Robot};
use kinematics::joints::Joints;
use linear_algebra::{Isometry2, Isometry3};
use ros_z::{
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::Time as RosTime,
};
use types::{
    ball_filter_tuning::{NAMESPACE, PROGRESS_TOPIC, Progress, ROUTER},
    ball_position::BallPosition,
    time_wrapper::TimeWrapper,
};

use crate::{
    bevy_mujoco::{
        MujocoModelUpdateSet, MujocoWorld, MujocoWorldPlugin, SimulationMode, from_mujoco,
    },
    parameters::{CurrentSimulatorParameters, SimulatorParameters},
    robot_io::RobotBinding,
    scene::{field::FieldPlugin, robot, visual::ObjectVisualAssets},
};

#[derive(Default)]
struct Snapshot {
    joints: Option<(RosTime, Joints<MotorState>)>,
    torso: Option<TimeWrapper<Option<Isometry3<Field, Robot>>>>,
    balls: Option<TimeWrapper<Vec<nalgebra::Isometry3<f32>>>>,
    ball_velocities: Option<TimeWrapper<Vec<nalgebra::Vector3<f32>>>>,
    obstacles: Option<TimeWrapper<Vec<nalgebra::Point3<f32>>>>,
    estimate: Option<(RosTime, Option<BallPosition<Ground>>)>,
    ground_to_field: BTreeMap<RosTime, Isometry2<Ground, Field>>,
    progress: Option<Progress>,
    parameters: Option<Arc<SimulatorParameters>>,
    received_at: Option<Instant>,
}
#[derive(Resource)]
struct ViewerData(Arc<Mutex<Snapshot>>);
#[derive(Component)]
struct ViewedRobot;
#[derive(Component)]
struct ViewedBall(usize);
#[derive(Component)]
struct ViewedObstacle(usize);
#[derive(Component)]
struct StatusText;

pub fn run() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let data = Arc::new(Mutex::new(Snapshot::default()));
    let state = data.clone();
    let (context, task) = runtime.block_on(async {
        let context = ContextBuilder::default().with_mode("client")
            .with_router_endpoint(ROUTER)?.with_namespace(NAMESPACE)
            .disable_multicast_scouting().build().await?;
        let node = context.create_node(format!("tuning_viewer_{}", std::process::id())).build().await?;
        let joints = node.subscriber::<Joints<MotorState>>("inputs/serial_motor_states").build().await?;
        let torso = node.subscriber::<TimeWrapper<Option<Isometry3<Field, Robot>>>>("localization/pose_3d").build().await?;
        let balls = node.subscriber::<TimeWrapper<Vec<nalgebra::Isometry3<f32>>>>("simulation/ball_poses_world").build().await?;
        let ball_velocities = node.subscriber::<TimeWrapper<Vec<nalgebra::Vector3<f32>>>>("simulation/ball_velocities_world").build().await?;
        let obstacles = node.subscriber::<TimeWrapper<Vec<nalgebra::Point3<f32>>>>("simulation/obstacle_positions_world").build().await?;
        let estimate = node.subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position").build().await?;
        let ground_to_field = node.subscriber::<Isometry2<Ground, Field>>("ground_to_field").build().await?;
        let progress = node.subscriber::<Progress>(PROGRESS_TOPIC).qos(QosProfile {
            durability: QosDurability::TransientLocal, history: QosHistory::from_depth(1), ..Default::default()
        }).build().await?;
        let parameters = node.subscriber::<SimulatorParameters>("simulation/parameters").qos(QosProfile {
            durability: QosDurability::TransientLocal, history: QosHistory::from_depth(1), ..Default::default()
        }).build().await?;
        let task = tokio::spawn(async move {
            let _node = node;
            loop {
                tokio::select! {
                    message = parameters.recv() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").parameters = Some(Arc::new(message));
                    }
                    message = joints.recv_with_metadata() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").joints = Some((message.source_time, message.into_message()));
                    }
                    message = torso.recv() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").torso = Some(message);
                    }
                    message = balls.recv() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").balls = Some(message);
                    }
                    message = ball_velocities.recv() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").ball_velocities = Some(message);
                    }
                    message = obstacles.recv() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").obstacles = Some(message);
                    }
                    message = estimate.recv_with_metadata() => {
                        let Ok(message) = message else { break; };
                        state.lock().expect("viewer state lock").estimate = Some((message.source_time, message.into_message()));
                    }
                    message = ground_to_field.recv_with_metadata() => {
                        let Ok(message) = message else { break; };
                        let mut state = state.lock().expect("viewer state lock");
                        state.ground_to_field.insert(message.source_time, message.into_message());
                        while state.ground_to_field.len() > 512 { state.ground_to_field.pop_first(); }
                    }
                    message = progress.recv() => {
                        let Ok(message) = message else { break; };
                        let mut state = state.lock().expect("viewer state lock");
                        if state.progress.as_ref().is_some_and(|old| old.output_directory != message.output_directory) {
                            state.ground_to_field.clear();
                            state.estimate = None;
                        }
                        state.progress = Some(message);
                        state.received_at = Some(Instant::now());
                    }
                }
            }
        });
        Ok::<_, color_eyre::Report>((context, task))
    })?;
    let parameters: SimulatorParameters = json5::from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/parameters/simulator.json5"
    ))?)?;
    let mut app = App::new();
    app.insert_resource(ViewerData(data))
        .insert_resource(CurrentSimulatorParameters {
            revision: 0,
            parameters: Arc::new(parameters),
        })
        .insert_resource(SimulationMode::Paused)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Ball tuning — live 3D view".into(),
                        resolution: (1280, 800).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                    ..default()
                }),
        )
        .init_resource::<ObjectVisualAssets>()
        .add_plugins((MujocoWorldPlugin, FieldPlugin, FreeCameraPlugin))
        .configure_sets(
            PreUpdate,
            crate::scene::SceneParameterUpdateSet.before(MujocoModelUpdateSet),
        )
        .add_systems(
            PreUpdate,
            (
                crate::scene::ball::update_ball_dimensions,
                crate::scene::goal::update_goal_dimensions,
            )
                .in_set(crate::scene::SceneParameterUpdateSet),
        )
        .add_systems(
            Startup,
            (
                crate::setup_scene,
                spawn_view,
                spawn_ball_model,
                spawn_obstacles,
            ),
        )
        .add_systems(
            PreUpdate,
            (show_recorded_pose, show_ball_model, show_obstacles)
                .chain()
                .after(MujocoModelUpdateSet),
        );
    app.run();
    drop(app);
    task.abort();
    context.shutdown()?;
    runtime.shutdown_timeout(Duration::from_secs(2));
    Ok(())
}

fn spawn_view(mut commands: Commands, assets: Res<ObjectVisualAssets>) {
    for index in 0..3 {
        let ball = assets.ball.spawn_visual(
            &mut commands,
            Transform::default(),
            false,
            RenderLayers::default(),
        );
        commands
            .entity(ball)
            .insert((ViewedBall(index), Visibility::Hidden));
    }
    commands.spawn((
        StatusText,
        Text::new("Waiting for optimizer sensor messages..."),
        TextFont {
            font_size: bevy::text::FontSize::Px(13.0),
            ..default()
        },
        bevy::prelude::Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cylinder::new(
        super::tuning_obstacles::RADIUS,
        super::tuning_obstacles::HEIGHT,
    ));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.05),
        ..default()
    });
    for index in 0..super::tuning_obstacles::COUNT {
        commands.spawn((
            ViewedObstacle(index),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    }
}

fn show_obstacles(
    input: Res<ViewerData>,
    mut obstacles: Query<(&ViewedObstacle, &mut Transform, &mut Visibility)>,
) {
    let state = input.0.lock().expect("viewer state lock");
    let positions = state.obstacles.as_ref().filter(|positions| {
        state.torso.as_ref().is_some_and(|torso| {
            torso.time.as_nanos().abs_diff(positions.time.as_nanos()) <= 100_000_000
        })
    });
    for (index, mut transform, mut visibility) in &mut obstacles {
        if let Some(p) = positions.and_then(|positions| positions.inner.get(index.0)) {
            *transform = Transform::from_xyz(p.x, p.z, -p.y);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn show_recorded_pose(
    input: Res<ViewerData>,
    mut parameters: ResMut<CurrentSimulatorParameters>,
    mut world: ResMut<MujocoWorld>,
    robots: Query<Entity, With<ViewedRobot>>,
    mut commands: Commands,
    assets: Res<ObjectVisualAssets>,
    mut balls: Query<(&ViewedBall, &mut Transform, &mut Visibility)>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
    let state = input.0.lock().expect("viewer state lock");
    if let Some(remote) = &state.parameters {
        if !Arc::ptr_eq(remote, &parameters.parameters) {
            parameters.parameters = remote.clone();
            parameters.revision += 1;
        }
    }
    let status = state
        .progress
        .as_ref()
        .map(|p| {
            if let Some(live) = &p.live_status {
                return format!("{} | {} | {}", p.status, live, p.phase.replace('·', "/"));
            }
            format!(
                "{} | recording {}/{} | {}",
                p.status,
                p.recording_index,
                p.recordings,
                p.phase.replace('·', "/")
            )
        })
        .unwrap_or_else(|| "Waiting for optimizer".into());
    let status = if state.joints.is_none() {
        format!("{status} | waiting for sensor messages")
    } else if state
        .progress
        .as_ref()
        .is_some_and(|p| p.status != "Recording" && p.live_status.is_none())
    {
        format!("{status} | last captured pose (no live capture)")
    } else {
        status
    };
    let stale = state
        .received_at
        .is_none_or(|time| time.elapsed() > Duration::from_secs(3));
    **text = Text::new(format!(
        "{status}{}\nRead-only view | hold right mouse + WASD to move | Q down / E up\nBlue ball / arrow: filter | original balls / green arrows: truth | arrows: 1 m per m/s\nOrange cylinders: moving opponents that block camera detections",
        if stale { " | no live updates" } else { "" }
    ));
    let (Some((joint_time, joints)), Some(torso)) = (&state.joints, &state.torso) else {
        return;
    };
    let Some(robot) = robots.iter().next() else {
        let entity = robot::spawn(&mut commands, &assets.robot, Transform::default());
        commands.entity(entity).insert(ViewedRobot);
        return;
    };
    if !world.contains_object(robot)
        || joint_time.as_nanos().abs_diff(torso.time.as_nanos()) > 20_000_000
    {
        return;
    }
    let Some(pose) = torso.inner.map(|pose| pose.inverse()) else {
        return;
    };
    let q = pose.inner.rotation.quaternion();
    let p = pose.inner.translation.vector;
    let transform = from_mujoco(
        [p.x as f64, p.y as f64, p.z as f64],
        [q.w as f64, q.i as f64, q.j as f64, q.k as f64],
    );
    if let Err(error) = world.set_object_pose(robot, transform) {
        warn!("{error}");
        return;
    }
    let result = RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))
        .and_then(|binding| binding.set_measured_joints(world.data_mut(), joints));
    if let Err(error) = result {
        warn!("{error:#}");
        return;
    }
    world.data_mut().forward();
    for (index, mut transform, mut visibility) in &mut balls {
        let ball_pose = state
            .balls
            .as_ref()
            .filter(|balls| balls.time.as_nanos().abs_diff(torso.time.as_nanos()) <= 100_000_000)
            .and_then(|balls| balls.inner.get(index.0));
        if let Some(pose) = ball_pose {
            let p = pose.translation.vector;
            let q = pose.rotation.quaternion();
            *transform = from_mujoco(
                [f64::from(p.x), f64::from(p.y), f64::from(p.z)],
                [
                    f64::from(q.w),
                    f64::from(q.i),
                    f64::from(q.j),
                    f64::from(q.k),
                ],
            );
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[derive(Component)]
enum BallModelPart {
    Position,
    Shaft,
    Tip,
    TruthShaft(usize),
    TruthTip(usize),
}

fn spawn_ball_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<ObjectVisualAssets>,
) {
    let blue = Color::srgb(0.1, 0.55, 1.0);
    let mut marker = materials
        .get(&assets.ball.material(false))
        .expect("ball material loaded")
        .clone();
    marker.base_color = blue;
    let marker = materials.add(marker);
    let ball = assets.ball.spawn_visual(
        &mut commands,
        Transform::default(),
        false,
        RenderLayers::default(),
    );
    commands.entity(ball).insert((
        BallModelPart::Position,
        MeshMaterial3d(marker),
        Visibility::Hidden,
        bevy::light::NotShadowCaster,
    ));
    let arrow = materials.add(StandardMaterial {
        base_color: blue,
        unlit: true,
        ..default()
    });
    let truth_arrow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 1.0, 0.3),
        unlit: true,
        ..default()
    });
    let shaft = meshes.add(Cylinder::new(1.0, 1.0));
    let tip = meshes.add(Cone {
        radius: 1.0,
        height: 1.0,
    });
    let mut arrows = vec![
        (BallModelPart::Shaft, shaft.clone(), arrow.clone()),
        (BallModelPart::Tip, tip.clone(), arrow),
    ];
    for index in 0..3 {
        arrows.push((
            BallModelPart::TruthShaft(index),
            shaft.clone(),
            truth_arrow.clone(),
        ));
        arrows.push((
            BallModelPart::TruthTip(index),
            tip.clone(),
            truth_arrow.clone(),
        ));
    }
    for (part, mesh, material) in arrows {
        commands.spawn((
            part,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
            Pickable::IGNORE,
        ));
    }
}

fn truth_arrow(state: &Snapshot, index: usize) -> Option<super::command_vectors::Arrow> {
    let time = state.torso.as_ref()?.time;
    let poses = state.balls.as_ref()?;
    let velocities = state.ball_velocities.as_ref()?;
    if poses.time.as_nanos().abs_diff(time.as_nanos()) > 100_000_000
        || velocities.time.as_nanos().abs_diff(poses.time.as_nanos()) > 20_000_000
    {
        return None;
    }
    let p = poses.inner.get(index)?.translation.vector;
    let v = velocities.inner.get(index)?;
    Some(super::command_vectors::Arrow {
        origin: Vec3::new(p.x, p.z, -p.y),
        vector: Vec3::new(v.x, v.z, -v.y),
    })
}

fn model_in_field(state: &Snapshot, time: RosTime) -> Option<BallPosition<Field>> {
    let (estimate_time, estimate) = state.estimate?;
    if estimate_time.as_nanos().abs_diff(time.as_nanos()) > 100_000_000 {
        return None;
    }
    let estimate = estimate?;
    let before = state.ground_to_field.range(..=estimate_time).next_back();
    let after = state.ground_to_field.range(estimate_time..).next();
    let (pose_time, pose) = before
        .into_iter()
        .chain(after)
        .min_by_key(|(time, _)| time.as_nanos().abs_diff(estimate_time.as_nanos()))?;
    if pose_time.as_nanos().abs_diff(estimate_time.as_nanos()) > 20_000_000 {
        return None;
    }
    Some(*pose * estimate)
}

fn show_ball_model(
    input: Res<ViewerData>,
    parameters: Res<CurrentSimulatorParameters>,
    mut parts: Query<(&BallModelPart, &mut Transform, &mut Visibility)>,
) {
    use super::command_vectors::{Arrow, part_transform};
    let state = input.0.lock().expect("viewer state lock");
    let model = state
        .torso
        .as_ref()
        .and_then(|torso| model_in_field(&state, torso.time));
    let radius = parameters.parameters.field_dimensions.ball_radius;
    for (part, mut transform, mut visibility) in &mut parts {
        let next = if let BallModelPart::TruthShaft(index) | BallModelPart::TruthTip(index) = part {
            truth_arrow(&state, *index)
                .and_then(|arrow| part_transform(arrow, matches!(part, BallModelPart::TruthTip(_))))
        } else {
            model.and_then(|model| {
                let origin = Vec3::new(model.position.x(), radius, -model.position.y());
                if !origin.is_finite() {
                    return None;
                }
                match part {
                    BallModelPart::Position => {
                        // Slightly larger so coincident physical/model balls remain distinguishable.
                        Some(Transform::from_translation(origin).with_scale(Vec3::splat(1.04)))
                    }
                    BallModelPart::Shaft | BallModelPart::Tip => part_transform(
                        Arrow {
                            origin,
                            vector: Vec3::new(model.velocity.x(), 0.0, -model.velocity.y()),
                        },
                        matches!(part, BallModelPart::Tip),
                    ),
                    BallModelPart::TruthShaft(_) | BallModelPart::TruthTip(_) => None,
                }
            })
        };
        if let Some(next) = next {
            *transform = next;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{point, vector};

    #[test]
    fn truth_velocity_uses_world_axes_and_requires_a_present_synchronized_ball() {
        let time = RosTime::from_nanos(1_000_000_000);
        let mut state = Snapshot {
            torso: Some(TimeWrapper { time, inner: None }),
            balls: Some(TimeWrapper {
                time,
                inner: vec![nalgebra::Isometry3::translation(2.0, 3.0, 0.1)],
            }),
            ball_velocities: Some(TimeWrapper {
                time,
                inner: vec![nalgebra::vector![4.0, -2.0, 0.5]],
            }),
            ..Default::default()
        };
        let arrow = truth_arrow(&state, 0).unwrap();
        assert_eq!(arrow.origin, Vec3::new(2.0, 0.1, -3.0));
        assert_eq!(arrow.vector, Vec3::new(4.0, 0.5, 2.0));
        state.ball_velocities.as_mut().unwrap().time = RosTime::from_nanos(1_021_000_000);
        assert!(truth_arrow(&state, 0).is_none());
        state.ball_velocities.as_mut().unwrap().time = time;
        state.balls.as_mut().unwrap().inner.clear();
        assert!(truth_arrow(&state, 0).is_none());
    }

    #[test]
    fn ball_model_uses_its_own_pose_timestamp_and_rejects_stale_data() {
        let time = RosTime::from_nanos(1_000_000_000);
        let mut state = Snapshot::default();
        state.estimate = Some((
            time,
            Some(BallPosition {
                position: point![1.0, 0.0],
                velocity: vector![0.5, 0.0],
                last_seen: time,
            }),
        ));
        state.ground_to_field.insert(
            time,
            Isometry2::from_parts(vector![2.0, 3.0], std::f32::consts::FRAC_PI_2),
        );
        let later = RosTime::from_nanos(1_040_000_000);
        state
            .ground_to_field
            .insert(later, Isometry2::from_parts(vector![20.0, 30.0], 0.0));
        let model = model_in_field(&state, later).unwrap();
        assert!((model.position - point![2.0, 4.0]).norm() < 1e-5);
        assert!((model.velocity - vector![0.0, 0.5]).norm() < 1e-5);
        assert!(model_in_field(&state, RosTime::from_nanos(1_101_000_000)).is_none());
        state.ground_to_field.remove(&time);
        assert!(
            model_in_field(&state, later).is_none(),
            "40 ms transform must be rejected"
        );
        state.estimate = Some((later, None));
        assert!(
            model_in_field(&state, later).is_none(),
            "lost tracks must disappear"
        );
    }
}
