use std::{collections::HashMap, path::absolute};

use bevy::prelude::*;
use communication::{
    client::{protocol::SubscriptionEvent, BinarySubscriptionHandle, Client, ClientHandle, Status},
    messages::Path,
};
use coordinate_systems::{Field, Ground, Robot};
use geometry::line_segment::LineSegment;
use linear_algebra::{vector, Isometry2, Isometry3, Orientation3, Vector3};
use repository::Repository;
use serde::Deserialize;
use types::{
    ball_position::BallPosition,
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
};
use urdf_rs::Geometry;

use crate::{async_runtime::AsyncRuntime, ball::BallAssets, parameters::Parameters};

pub struct NaoPlugin;

impl Plugin for NaoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_robot)
            .add_systems(Update, handle_communication)
            .add_systems(Update, update_robot_transform)
            .add_systems(Update, update_joints)
            .add_systems(Update, update_ball)
            .add_systems(Update, update_lines)
            .add_event::<SpawnRobot>()
            .add_systems(Update, spawn_robot);
    }
}

pub struct Visual {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub origin: Transform,
}

pub struct Link {
    pub name: String,
    pub visuals: Vec<Visual>,
}

pub struct Joint {
    pub name: String,
    pub parent: String,
    pub child: String,
    pub origin: Transform,
}

#[derive(Resource)]
pub struct RobotSpecification {
    pub links: HashMap<String, Link>,
    pub joints: HashMap<String, Joint>,
}

fn setup_robot(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let repository = Repository::find_root(absolute(".").unwrap()).unwrap();
    let urdf = urdf_rs::read_file(repository.root.join("tools/mio/assets/NAO.urdf")).unwrap();
    let fallback_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 1., 1.),
        ..default()
    });
    let links = urdf
        .links
        .into_iter()
        .map(|link| {
            let name = link.name.clone();
            let visuals = link
                .visual
                .into_iter()
                .map(|visual| {
                    let (mesh, scale) = match visual.geometry {
                        Geometry::Mesh { filename, scale } => (server.load(filename), scale),
                        _ => unimplemented!("only mesh geometry is supported"),
                    };
                    let scale = scale
                        .map(|scale| Vec3::new(scale[0] as f32, scale[1] as f32, scale[2] as f32))
                        .unwrap_or(Vec3::ONE);
                    let material = match visual.material {
                        Some(urdf_rs::Material {
                            texture: Some(urdf_rs::Texture { filename }),
                            ..
                        }) => server.load(filename),
                        Some(urdf_rs::Material {
                            color: Some(urdf_rs::Color { rgba }),
                            ..
                        }) => materials.add(StandardMaterial {
                            base_color: Color::srgba(
                                rgba.0[0] as f32,
                                rgba.0[1] as f32,
                                rgba.0[2] as f32,
                                rgba.0[3] as f32,
                            ),
                            ..default()
                        }),
                        _ => fallback_material.clone(),
                    };
                    let position = visual.origin.xyz;
                    let rotation = visual.origin.rpy;
                    let origin = Transform::from_xyz(
                        position[0] as f32,
                        position[1] as f32,
                        position[2] as f32,
                    )
                    .with_rotation(Quat::from_euler(
                        EulerRot::ZYX,
                        rotation[2] as f32,
                        rotation[1] as f32,
                        rotation[0] as f32,
                    ))
                    .with_scale(scale);
                    Visual {
                        mesh,
                        material,
                        origin,
                    }
                })
                .collect();
            (name.clone(), Link { name, visuals })
        })
        .collect();
    let joints = urdf
        .joints
        .into_iter()
        .map(|joint| {
            let name = joint.name.clone();
            let parent = joint.parent.link.clone();
            let child = joint.child.link.clone();
            let translation = joint.origin.xyz;
            let translation = Vec3::new(
                translation[0] as f32,
                translation[1] as f32,
                translation[2] as f32,
            );
            let rotation = joint.origin.rpy;
            let rotation = Quat::from_euler(
                EulerRot::ZYX,
                rotation[2] as f32,
                rotation[1] as f32,
                rotation[0] as f32,
            );
            let origin = Transform::from_translation(translation).with_rotation(rotation);
            (
                name.clone(),
                Joint {
                    name,
                    parent,
                    child,
                    origin,
                },
            )
        })
        .collect();
    commands.insert_resource(RobotSpecification { links, joints });
}

struct Subscription<T> {
    receiver: BinarySubscriptionHandle,
    value: Option<T>,
}

impl<T> Subscription<T> {
    async fn new(communication: &ClientHandle, output: impl Into<Path>) -> Self {
        let receiver = communication.subscribe_binary(output).await;
        Self {
            receiver,
            value: None,
        }
    }
}

impl<T> Subscription<T>
where
    for<'de> T: Deserialize<'de>,
{
    fn update(&mut self) {
        while let Ok(message) = self.receiver.receiver.try_recv() {
            match message.as_ref() {
                SubscriptionEvent::Successful { value, .. }
                | SubscriptionEvent::Update { value, .. } => {
                    self.value = bincode::deserialize(value).ok()
                }
                SubscriptionEvent::Failure { error } => eprintln!("{error}"),
            };
        }
    }
}

#[derive(Component)]
pub struct Nao {
    pub address: String,
    pub connected: bool,
    pub client: ClientHandle,
    sensor_data: Subscription<Joints<f32>>,
    robot_to_ground: Subscription<Option<Isometry3<Robot, Ground>>>,
    ground_to_field: Subscription<Option<Isometry2<Ground, Field>>>,
    ball_position: Subscription<Option<BallPosition<Ground>>>,
    lines_in_ground_bottom: Subscription<Vec<LineSegment<Ground>>>,
    lines_in_ground_top: Subscription<Vec<LineSegment<Ground>>>,
    ball: Entity,
    joints: Joints<Entity>,
}

impl Nao {
    pub async fn new(links: &HashMap<String, Entity>, ball: Entity) -> Self {
        let (client_object, client) = Client::new(String::new());
        tokio::spawn(client_object.run());
        let sensor_data =
            Subscription::new(&client, "Control.main_outputs.sensor_data.positions").await;
        let robot_to_ground =
            Subscription::new(&client, "Control.main_outputs.robot_to_ground").await;
        let ground_to_field =
            Subscription::new(&client, "Control.main_outputs.ground_to_field").await;
        let ball_position = Subscription::new(&client, "Control.main_outputs.ball_position").await;
        let lines_in_ground_bottom =
            Subscription::new(&client, "VisionBottom.main_outputs.line_data.lines").await;
        let lines_in_ground_top =
            Subscription::new(&client, "VisionTop.main_outputs.line_data.lines").await;
        let joints = Joints {
            head: HeadJoints {
                yaw: *links.get("HeadYaw_link").unwrap(),
                pitch: *links.get("HeadPitch_link").unwrap(),
            },
            left_arm: ArmJoints {
                shoulder_pitch: *links.get("LShoulderPitch_link").unwrap(),
                shoulder_roll: *links.get("LShoulderRoll_link").unwrap(),
                elbow_yaw: *links.get("LElbowYaw_link").unwrap(),
                elbow_roll: *links.get("LElbowRoll_link").unwrap(),
                wrist_yaw: *links.get("LWristYaw_link").unwrap(),
                hand: Entity::PLACEHOLDER,
            },
            right_arm: ArmJoints {
                shoulder_pitch: *links.get("RShoulderPitch_link").unwrap(),
                shoulder_roll: *links.get("RShoulderRoll_link").unwrap(),
                elbow_yaw: *links.get("RElbowYaw_link").unwrap(),
                elbow_roll: *links.get("RElbowRoll_link").unwrap(),
                wrist_yaw: *links.get("RWristYaw_link").unwrap(),
                hand: Entity::PLACEHOLDER,
            },
            left_leg: LegJoints {
                hip_yaw_pitch: *links.get("LHipYawPitch_link").unwrap(),
                hip_roll: *links.get("LHipRoll_link").unwrap(),
                hip_pitch: *links.get("LHipPitch_link").unwrap(),
                knee_pitch: *links.get("LKneePitch_link").unwrap(),
                ankle_pitch: *links.get("LAnklePitch_link").unwrap(),
                ankle_roll: *links.get("LAnkleRoll_link").unwrap(),
            },
            right_leg: LegJoints {
                hip_yaw_pitch: *links.get("RHipYawPitch_link").unwrap(),
                hip_roll: *links.get("RHipRoll_link").unwrap(),
                hip_pitch: *links.get("RHipPitch_link").unwrap(),
                knee_pitch: *links.get("RKneePitch_link").unwrap(),
                ankle_pitch: *links.get("RAnklePitch_link").unwrap(),
                ankle_roll: *links.get("RAnkleRoll_link").unwrap(),
            },
        };
        Self {
            client,
            address: String::new(),
            connected: false,
            sensor_data,
            robot_to_ground,
            ground_to_field,
            ball_position,
            joints,
            ball,
            lines_in_ground_bottom,
            lines_in_ground_top,
        }
    }
}

fn handle_communication(mut naos: Query<(&Nao, &mut Visibility)>, runtime: Res<AsyncRuntime>) {
    for (nao, mut visibility) in naos.iter_mut() {
        let status = runtime.runtime.block_on(nao.client.status());
        *visibility = match status {
            Status::Disconnected | Status::Connecting => Visibility::Hidden,
            Status::Connected => Visibility::Visible,
        }
    }
}

#[derive(Event)]
pub struct SpawnRobot;

pub fn spawn_robot(
    mut commands: Commands,
    mut spawn_robot: EventReader<SpawnRobot>,
    robot_specification: Res<RobotSpecification>,
    runtime: Res<AsyncRuntime>,
    ball_assets: Res<BallAssets>,
) {
    for _ in spawn_robot.read() {
        let ball = commands
            .spawn((
                Name::new("ball"),
                Visibility::Hidden,
                Mesh3d(ball_assets.mesh.clone()),
                MeshMaterial3d(ball_assets.material.clone()),
            ))
            .id();
        let mut links = HashMap::new();
        commands
            .spawn((Name::new("robot"), Visibility::Hidden, Transform::default()))
            .with_children(|builder| {
                for link in robot_specification.links.values() {
                    let id = builder
                        .spawn((
                            Name::new(link.name.clone()),
                            Transform::default(),
                            Visibility::default(),
                        ))
                        .with_children(|builder| {
                            for visual in link.visuals.iter() {
                                builder.spawn((
                                    Mesh3d(visual.mesh.clone()),
                                    MeshMaterial3d(visual.material.clone()),
                                    visual.origin,
                                ));
                            }
                        })
                        .id();
                    links.insert(link.name.clone(), id);
                }
            })
            .insert(runtime.runtime.block_on(Nao::new(&links, ball)));
        for joint in robot_specification.joints.values() {
            let parent = links.get(&joint.parent).unwrap();
            let child = links.get(&joint.child).unwrap();
            let joint_id = commands
                .spawn((
                    Name::new(joint.name.clone()),
                    joint.origin,
                    Visibility::default(),
                ))
                .add_child(*child)
                .id();
            commands.entity(*parent).add_child(joint_id);
        }
    }
}

const FALLBACK_ROBOT_HEIGHT: f32 = 0.5;

fn update_robot_transform(mut naos: Query<(&mut Nao, &mut Transform)>) {
    for (mut nao, mut transform) in naos.iter_mut() {
        nao.robot_to_ground.update();
        let robot_to_ground = nao
            .robot_to_ground
            .value
            .unwrap_or_default()
            .unwrap_or(Isometry3::from_translation(0., 0., FALLBACK_ROBOT_HEIGHT));
        nao.ground_to_field.update();
        let ground_to_field = nao
            .ground_to_field
            .value
            .unwrap_or_default()
            .unwrap_or_default();
        let ground_to_field_3 = Isometry3::<Ground, Field>::from_parts(
            vector![
                ground_to_field.translation().x(),
                ground_to_field.translation().y(),
                0.
            ],
            Orientation3::new(Vector3::z_axis() * ground_to_field.orientation().angle()),
        );
        let robot_to_field = ground_to_field_3 * robot_to_ground;
        transform.translation = Vec3::new(
            robot_to_field.translation().x(),
            robot_to_field.translation().y(),
            robot_to_field.translation().z(),
        );
        let quaternion_coords = robot_to_field.inner.rotation.coords;
        transform.rotation = Quat::from_xyzw(
            quaternion_coords[0],
            quaternion_coords[1],
            quaternion_coords[2],
            quaternion_coords[3],
        );
    }
}

fn update_joints(mut robots: Query<&mut Nao>, mut transforms: Query<&mut Transform>) {
    for mut robot in robots.iter_mut() {
        robot.sensor_data.update();
        let joints = robot.sensor_data.value.unwrap_or_default();
        let mut head_yaw = transforms.get_mut(robot.joints.head.yaw).unwrap();
        head_yaw.rotation = Quat::from_rotation_z(joints.head.yaw);
        let mut head_pitch = transforms.get_mut(robot.joints.head.pitch).unwrap();
        head_pitch.rotation = Quat::from_rotation_z(joints.head.pitch);

        let mut left_shoulder_pitch = transforms
            .get_mut(robot.joints.left_arm.shoulder_pitch)
            .unwrap();
        left_shoulder_pitch.rotation = Quat::from_rotation_z(joints.left_arm.shoulder_pitch);
        let mut left_shoulder_roll = transforms
            .get_mut(robot.joints.left_arm.shoulder_roll)
            .unwrap();
        left_shoulder_roll.rotation = Quat::from_rotation_z(joints.left_arm.shoulder_roll);
        let mut left_elbow_yaw = transforms.get_mut(robot.joints.left_arm.elbow_yaw).unwrap();
        left_elbow_yaw.rotation = Quat::from_rotation_z(joints.left_arm.elbow_yaw);
        let mut left_elbow_roll = transforms
            .get_mut(robot.joints.left_arm.elbow_roll)
            .unwrap();
        left_elbow_roll.rotation = Quat::from_rotation_z(joints.left_arm.elbow_roll);
        let mut left_wrist_yaw = transforms.get_mut(robot.joints.left_arm.wrist_yaw).unwrap();
        left_wrist_yaw.rotation = Quat::from_rotation_z(joints.left_arm.wrist_yaw);

        let mut right_shoulder_pitch = transforms
            .get_mut(robot.joints.right_arm.shoulder_pitch)
            .unwrap();
        right_shoulder_pitch.rotation = Quat::from_rotation_z(joints.right_arm.shoulder_pitch);
        let mut right_shoulder_roll = transforms
            .get_mut(robot.joints.right_arm.shoulder_roll)
            .unwrap();
        right_shoulder_roll.rotation = Quat::from_rotation_z(joints.right_arm.shoulder_roll);
        let mut right_elbow_yaw = transforms
            .get_mut(robot.joints.right_arm.elbow_yaw)
            .unwrap();
        right_elbow_yaw.rotation = Quat::from_rotation_z(joints.right_arm.elbow_yaw);
        let mut right_elbow_roll = transforms
            .get_mut(robot.joints.right_arm.elbow_roll)
            .unwrap();
        right_elbow_roll.rotation = Quat::from_rotation_z(joints.right_arm.elbow_roll);
        let mut right_wrist_yaw = transforms
            .get_mut(robot.joints.right_arm.wrist_yaw)
            .unwrap();
        right_wrist_yaw.rotation = Quat::from_rotation_z(joints.right_arm.wrist_yaw);

        let mut left_hip_yaw_pitch = transforms
            .get_mut(robot.joints.left_leg.hip_yaw_pitch)
            .unwrap();
        left_hip_yaw_pitch.rotation = Quat::from_rotation_z(joints.left_leg.hip_yaw_pitch);
        let mut left_hip_roll = transforms.get_mut(robot.joints.left_leg.hip_roll).unwrap();
        left_hip_roll.rotation = Quat::from_rotation_z(joints.left_leg.hip_roll);
        let mut left_hip_pitch = transforms.get_mut(robot.joints.left_leg.hip_pitch).unwrap();
        left_hip_pitch.rotation = Quat::from_rotation_z(joints.left_leg.hip_pitch);
        let mut left_knee_pitch = transforms
            .get_mut(robot.joints.left_leg.knee_pitch)
            .unwrap();
        left_knee_pitch.rotation = Quat::from_rotation_z(joints.left_leg.knee_pitch);
        let mut left_ankle_pitch = transforms
            .get_mut(robot.joints.left_leg.ankle_pitch)
            .unwrap();
        left_ankle_pitch.rotation = Quat::from_rotation_z(joints.left_leg.ankle_pitch);
        let mut left_ankle_roll = transforms
            .get_mut(robot.joints.left_leg.ankle_roll)
            .unwrap();
        left_ankle_roll.rotation = Quat::from_rotation_z(joints.left_leg.ankle_roll);

        let mut right_hip_yaw_pitch = transforms
            .get_mut(robot.joints.right_leg.hip_yaw_pitch)
            .unwrap();
        right_hip_yaw_pitch.rotation = Quat::from_rotation_z(joints.right_leg.hip_yaw_pitch);
        let mut right_hip_roll = transforms.get_mut(robot.joints.right_leg.hip_roll).unwrap();
        right_hip_roll.rotation = Quat::from_rotation_z(joints.right_leg.hip_roll);
        let mut right_hip_pitch = transforms
            .get_mut(robot.joints.right_leg.hip_pitch)
            .unwrap();
        right_hip_pitch.rotation = Quat::from_rotation_z(joints.right_leg.hip_pitch);
        let mut right_knee_pitch = transforms
            .get_mut(robot.joints.right_leg.knee_pitch)
            .unwrap();
        right_knee_pitch.rotation = Quat::from_rotation_z(joints.right_leg.knee_pitch);
        let mut right_ankle_pitch = transforms
            .get_mut(robot.joints.right_leg.ankle_pitch)
            .unwrap();
        right_ankle_pitch.rotation = Quat::from_rotation_z(joints.right_leg.ankle_pitch);
        let mut right_ankle_roll = transforms
            .get_mut(robot.joints.right_leg.ankle_roll)
            .unwrap();
        right_ankle_roll.rotation = Quat::from_rotation_z(joints.right_leg.ankle_roll);
    }
}

fn update_ball(
    mut naos: Query<&mut Nao>,
    mut balls: Query<(&mut Transform, &mut Visibility)>,
    parameters: Res<Parameters>,
) {
    let ball_radius = parameters.field_dimensions.ball_radius;
    for mut nao in naos.iter_mut() {
        nao.ball_position.update();
        nao.robot_to_ground.update();
        nao.ground_to_field.update();
        let ground_to_field = nao.ground_to_field.value.flatten().unwrap_or_default();
        let (mut transform, mut visibility) = balls.get_mut(nao.ball).unwrap();
        match nao.ball_position.value {
            Some(Some(ball_position)) => {
                let abs_ball = ground_to_field * ball_position.position;
                *transform = Transform::from_xyz(abs_ball.x(), abs_ball.y(), ball_radius);
                *visibility = Visibility::Visible;
            }
            _ => {
                *visibility = Visibility::Hidden;
            }
        };
    }
}

fn update_lines(mut naos: Query<&mut Nao>, mut gizmos: Gizmos) {
    for mut nao in naos.iter_mut() {
        nao.ground_to_field.update();
        nao.lines_in_ground_bottom.update();
        nao.lines_in_ground_top.update();
        let ground_to_field = nao.ground_to_field.value.flatten().unwrap_or_default();
        for line in nao
            .lines_in_ground_bottom
            .value
            .iter()
            .chain(nao.lines_in_ground_top.value.iter())
            .flatten()
        {
            let start = ground_to_field * line.0;
            let end = ground_to_field * line.1;
            gizmos.line(
                Vec3::new(start.x(), start.y(), 0.0),
                Vec3::new(end.x(), end.y(), 0.0),
                Srgba::RED,
            );
        }
    }
}
