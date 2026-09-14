use std::{
    collections::HashMap,
    f32::consts::FRAC_PI_2,
    ffi::{CStr, CString},
    path::PathBuf,
    ptr,
    sync::Arc,
    time::Duration,
};

use bevy::{platform::cell::SyncCell, prelude::*};
use mujoco_rs::{
    mujoco_c::{mj_recompile, mjData, mjModel, mjSpec, mjs_attach, mjs_getError, mjs_setDeepCopy},
    prelude::{MjData, MjModel, MjSpec, MjtGeom, MjtObj, SpecItem},
};

type MjcfFactory = Arc<dyn Fn() -> Result<MjSpec, String> + Send + Sync>;

#[derive(Clone, Component)]
pub struct MjcfObject {
    factory: MjcfFactory,
    root_body: String,
    pose_target: PoseTarget,
    grounded: bool,
    reapply_pose_on_change: bool,
}

impl std::fmt::Debug for MjcfObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MjcfObject")
            .field("root_body", &self.root_body)
            .field("pose_target", &self.pose_target)
            .field("grounded", &self.grounded)
            .field("reapply_pose_on_change", &self.reapply_pose_on_change)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
enum PoseTarget {
    Fixed,
    FreeJoint(String),
    MocapBody(String),
}

impl MjcfObject {
    pub fn new(path: impl Into<PathBuf>, root_body: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            factory: Arc::new(move || MjSpec::from_xml(&path).map_err(|error| error.to_string())),
            root_body: root_body.into(),
            pose_target: PoseTarget::Fixed,
            grounded: false,
            reapply_pose_on_change: false,
        }
    }

    pub fn from_factory<F>(factory: F, root_body: impl Into<String>) -> Self
    where
        F: Fn() -> Result<MjSpec, String> + Send + Sync + 'static,
    {
        Self {
            factory: Arc::new(factory),
            root_body: root_body.into(),
            pose_target: PoseTarget::Fixed,
            grounded: false,
            reapply_pose_on_change: false,
        }
    }

    pub fn with_free_joint(mut self, name: impl Into<String>) -> Self {
        self.pose_target = PoseTarget::FreeJoint(name.into());
        self
    }

    pub fn with_mocap_body(mut self, name: impl Into<String>) -> Self {
        self.pose_target = PoseTarget::MocapBody(name.into());
        self
    }

    pub fn grounded(mut self) -> Self {
        self.grounded = true;
        self
    }

    pub fn reapply_pose_on_change(mut self) -> Self {
        self.reapply_pose_on_change = true;
        self
    }
}

#[derive(Component, Debug)]
pub struct MujocoBody {
    name: String,
}

impl MujocoBody {
    pub fn new(object: Entity, name: &str) -> Self {
        Self {
            name: qualified_name(object, name),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Resource)]
pub enum SimulationMode {
    #[default]
    Running,
    Paused,
}

#[derive(Debug, Message)]
pub struct SetObjectPose {
    pub object: Entity,
    pub transform: Transform,
}

#[derive(Debug)]
struct ObjectBinding {
    root_body: String,
    pose_target: PoseTarget,
}

#[derive(Resource)]
pub struct MujocoWorld {
    spec: SyncCell<MjSpec>,
    data: Option<MjData<Box<MjModel>>>,
    objects: HashMap<Entity, ObjectBinding>,
    pub generation: u64,
}

impl Default for MujocoWorld {
    fn default() -> Self {
        let mut spec = new_spec();
        let model = spec.compile().expect("empty MuJoCo model should compile");

        Self {
            spec: SyncCell::new(spec),
            data: Some(MjData::new(Box::new(model))),
            objects: HashMap::new(),
            generation: 0,
        }
    }
}

impl MujocoWorld {
    pub fn data(&self) -> &MjData<Box<MjModel>> {
        self.data.as_ref().expect("MuJoCo model should be compiled")
    }

    pub fn data_mut(&mut self) -> &mut MjData<Box<MjModel>> {
        self.data.as_mut().expect("MuJoCo model should be compiled")
    }
    fn add_object(&mut self, entity: Entity, object: &MjcfObject) -> Result<(), String> {
        if self.objects.contains_key(&entity) {
            return Err(format!("physics object {entity:?} already exists"));
        }

        attach_mjcf(
            self.spec.get(),
            entity,
            (object.factory)()?,
            &object.root_body,
        )?;
        self.objects.insert(
            entity,
            ObjectBinding {
                root_body: qualified_name(entity, &object.root_body),
                pose_target: match &object.pose_target {
                    PoseTarget::Fixed => PoseTarget::Fixed,
                    PoseTarget::FreeJoint(name) => {
                        PoseTarget::FreeJoint(qualified_name(entity, name))
                    }
                    PoseTarget::MocapBody(name) => {
                        PoseTarget::MocapBody(qualified_name(entity, name))
                    }
                },
            },
        );
        Ok(())
    }

    fn remove_object(&mut self, entity: Entity) -> Result<bool, String> {
        let Some(binding) = self.objects.get(&entity) else {
            return Ok(false);
        };
        let spec = self.spec.get();
        let element = spec
            .body_mut(&binding.root_body)
            .ok_or_else(|| format!("MuJoCo body {:?} does not exist", binding.root_body))?
            .element_mut_pointer();
        unsafe { spec.delete_element(element) }.map_err(|error| error.to_string())?;
        self.objects.remove(&entity);
        Ok(true)
    }

    pub fn set_object_pose(&mut self, entity: Entity, transform: Transform) -> Result<(), String> {
        let binding = self
            .objects
            .get(&entity)
            .ok_or_else(|| format!("unknown physics object {entity:?}"))?;
        let target = binding.pose_target.clone();
        let data = self.data.as_mut().expect("MuJoCo model should be compiled");
        let (position, rotation) = to_mujoco(transform);

        match target {
            PoseTarget::Fixed => return Err(format!("physics object {entity:?} is fixed")),
            PoseTarget::FreeJoint(joint) => {
                let info = data
                    .joint(&joint)
                    .ok_or_else(|| format!("MuJoCo joint {joint:?} does not exist"))?;
                let mut view = info.view_mut(data);

                if view.qpos.len() != 7 || view.qvel.len() != 6 {
                    return Err(format!("MuJoCo joint {joint:?} is not free"));
                }

                view.qpos.copy_from_slice(&[
                    position[0],
                    position[1],
                    position[2],
                    rotation[0],
                    rotation[1],
                    rotation[2],
                    rotation[3],
                ]);
                view.qvel.fill(0.0);
            }
            PoseTarget::MocapBody(body) => {
                let body_id = data
                    .model()
                    .name_to_id(MjtObj::mjOBJ_BODY, &body)
                    .ok_or_else(|| format!("MuJoCo body {body:?} does not exist"))?;
                let mocap_id = usize::try_from(data.model().body_mocapid()[body_id])
                    .map_err(|_| format!("MuJoCo body {body:?} is not a mocap body"))?;
                data.mocap_pos_mut()[mocap_id] = position;
                data.mocap_quat_mut()[mocap_id] = rotation;
            }
        }

        data.forward();
        Ok(())
    }

    pub fn contains_object(&self, entity: Entity) -> bool {
        self.objects.contains_key(&entity)
    }

    fn ground_object(&mut self, entity: Entity, mut transform: Transform) -> Result<(), String> {
        self.set_object_pose(entity, transform)?;
        let minimum_z = self
            .minimum_object_z(entity)
            .ok_or_else(|| format!("physics object {entity:?} has no geometry"))?;
        transform.translation.y -= minimum_z as f32;
        self.set_object_pose(entity, transform)
    }

    fn minimum_object_z(&self, entity: Entity) -> Option<f64> {
        let binding = self.objects.get(&entity)?;
        let data = self.data.as_ref()?;
        let model = data.model();
        let root = model.name_to_id(MjtObj::mjOBJ_BODY, &binding.root_body)?;
        let parents = model.body_parentid();

        model
            .geom_bodyid()
            .iter()
            .enumerate()
            .filter_map(|(geom, &body)| {
                if model.geom_contype()[geom] == 0 && model.geom_conaffinity()[geom] == 0 {
                    return None;
                }
                let body = usize::try_from(body).ok()?;
                is_descendant(body, root, parents).then(|| {
                    let [cx, cy, cz, hx, hy, hz] = model.geom_aabb()[geom];
                    let [_, _, _, _, _, _, r20, r21, r22] = data.geom_xmat()[geom];
                    let center = data.geom_xpos()[geom][2] + r20 * cx + r21 * cy + r22 * cz;
                    center - r20.abs() * hx - r21.abs() * hy - r22.abs() * hz
                })
            })
            .filter(|height| height.is_finite())
            .reduce(f64::min)
    }

    fn body_pose(&self, body: &MujocoBody) -> Option<Transform> {
        let data = self.data.as_ref()?;
        let body = data.body(&body.name)?.view(data);
        Some(from_mujoco(
            [body.xpos[0], body.xpos[1], body.xpos[2]],
            [body.xquat[0], body.xquat[1], body.xquat[2], body.xquat[3]],
        ))
    }

    fn recompile(&mut self) {
        let result = unsafe {
            let data = self.data.as_mut().expect("MuJoCo data should exist");
            let model = data.model_mut().ffi_mut() as *mut mjModel;
            let data = data.ffi_mut() as *mut mjData;
            let spec = self.spec.get().ffi_mut() as *mut mjSpec;
            mj_recompile(spec, ptr::null(), model, data)
        };

        if result != 0 {
            std::mem::forget(self.data.take().expect("MuJoCo data should exist"));
            let error = unsafe { mjs_getError(self.spec.get().ffi_mut()) };
            let message = if error.is_null() {
                "unknown error".into()
            } else {
                unsafe { CStr::from_ptr(error) }.to_string_lossy()
            };
            panic!("MuJoCo model compilation failed: {message}");
        }

        self.data
            .as_mut()
            .expect("MuJoCo data should exist")
            .forward();
    }
}

pub struct MujocoWorldPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct MujocoModelUpdateSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct MujocoStepSet;

impl Plugin for MujocoWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MujocoWorld>()
            .init_resource::<SimulationMode>()
            .add_message::<SetObjectPose>()
            .configure_sets(PreUpdate, MujocoModelUpdateSet)
            .add_systems(PreUpdate, update_objects.in_set(MujocoModelUpdateSet))
            .add_systems(FixedUpdate, step_world.in_set(MujocoStepSet))
            .add_systems(Update, (apply_pose_commands, sync_bodies).chain());
    }
}

fn update_objects(
    mut removed: RemovedComponents<MjcfObject>,
    objects: Query<(Entity, Ref<MjcfObject>, Option<&Transform>)>,
    mut world: ResMut<MujocoWorld>,
    mut fixed_time: ResMut<Time<Fixed>>,
) {
    let mut changed = false;
    for entity in removed.read() {
        changed |= world
            .remove_object(entity)
            .unwrap_or_else(|error| panic!("failed to remove MJCF object: {error}"));
    }

    let mut poses_to_apply = Vec::new();
    for (entity, object, transform) in &objects {
        if !object.is_added() && !object.is_changed() {
            continue;
        }
        if object.is_changed() && !object.is_added() {
            world
                .remove_object(entity)
                .unwrap_or_else(|error| panic!("failed to replace MJCF object: {error}"));
        }
        world
            .add_object(entity, &object)
            .unwrap_or_else(|error| panic!("failed to add MJCF object: {error}"));
        if !matches!(object.pose_target, PoseTarget::Fixed)
            && (object.is_added() || object.reapply_pose_on_change)
        {
            poses_to_apply.push((
                entity,
                transform.copied().unwrap_or_default(),
                object.is_added() && object.grounded,
            ));
        }
        changed = true;
    }

    if !changed {
        return;
    }

    world.recompile();
    world.generation += 1;
    let timestep = world
        .data
        .as_ref()
        .expect("MuJoCo data should exist")
        .model_opt()
        .timestep;
    fixed_time.set_timestep(Duration::from_secs_f64(timestep));

    for (entity, transform, grounded) in poses_to_apply {
        let result = if grounded {
            world.ground_object(entity, transform)
        } else {
            world.set_object_pose(entity, transform)
        };
        result.unwrap_or_else(|error| panic!("failed to set initial object pose: {error}"));
    }
}

fn step_world(mut world: ResMut<MujocoWorld>, mode: Res<SimulationMode>) {
    if *mode == SimulationMode::Running {
        world
            .data
            .as_mut()
            .expect("MuJoCo data should exist")
            .step();
    }
}

fn apply_pose_commands(
    mut commands: MessageReader<SetObjectPose>,
    mut world: ResMut<MujocoWorld>,
    mode: Res<SimulationMode>,
) {
    for command in commands.read() {
        if *mode == SimulationMode::Paused {
            world
                .set_object_pose(command.object, command.transform)
                .unwrap_or_else(|error| warn!("{error}"));
        } else {
            warn!("pause the simulation before moving a physics object");
        }
    }
}

fn sync_bodies(
    world: Res<MujocoWorld>,
    mut bodies: Query<(&MujocoBody, &mut Transform, Option<&mut Visibility>)>,
) {
    for (body, mut transform, visibility) in &mut bodies {
        if let Some(pose) = world.body_pose(body) {
            *transform = pose;
            if let Some(mut visibility) = visibility {
                *visibility = Visibility::Visible;
            }
        }
    }
}

fn new_spec() -> MjSpec {
    let mut spec = MjSpec::new();
    unsafe {
        assert_eq!(mjs_setDeepCopy(spec.ffi_mut(), 1), 0);
    }
    let world = spec.world_body_mut();
    world
        .add_geom()
        .with_name("ground")
        .with_type(MjtGeom::mjGEOM_PLANE)
        .with_size([0.0, 0.0, 0.01]);
    world
        .add_frame()
        .set_name("object_attachments")
        .expect("attachment frame name should be unique");
    spec
}

fn attach_mjcf(
    spec: &mut MjSpec,
    entity: Entity,
    mut child: MjSpec,
    root_body: &str,
) -> Result<(), String> {
    let prefix = CString::new(format!("object_{}_", entity.to_bits()))
        .expect("generated prefix has no null byte");
    let parent = spec
        .frame_mut("object_attachments")
        .expect("attachment frame should exist")
        .element_mut_pointer();
    let child = child
        .body_mut(root_body)
        .ok_or_else(|| format!("MJCF root body {root_body:?} does not exist"))?
        .element_mut_pointer();
    let attached = unsafe { mjs_attach(parent, child, prefix.as_ptr(), c"".as_ptr()) };

    if attached.is_null() {
        let error = unsafe { mjs_getError(spec.ffi_mut()) };
        return Err(if error.is_null() {
            "failed to attach MJCF".to_owned()
        } else {
            unsafe { CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned()
        });
    }

    Ok(())
}

fn qualified_name(entity: Entity, name: &str) -> String {
    format!("object_{}_{name}", entity.to_bits())
}

fn is_descendant(mut body: usize, root: usize, parents: &[i32]) -> bool {
    for _ in 0..parents.len() {
        if body == root {
            return true;
        }
        let Ok(parent) = usize::try_from(parents[body]) else {
            return false;
        };
        if parent == body {
            return false;
        }
        body = parent;
    }
    false
}

pub(crate) fn from_mujoco(position: [f64; 3], rotation: [f64; 4]) -> Transform {
    let basis = Quat::from_rotation_x(-FRAC_PI_2);
    let rotation = Quat::from_xyzw(
        rotation[1] as f32,
        rotation[2] as f32,
        rotation[3] as f32,
        rotation[0] as f32,
    );

    Transform::from_translation(Vec3::new(
        position[0] as f32,
        position[2] as f32,
        -position[1] as f32,
    ))
    .with_rotation(basis * rotation * basis.inverse())
}

fn to_mujoco(transform: Transform) -> ([f64; 3], [f64; 4]) {
    let basis = Quat::from_rotation_x(-FRAC_PI_2);
    let rotation = (basis.inverse() * transform.rotation * basis).normalize();
    let position = transform.translation;

    (
        [position.x as f64, -position.z as f64, position.y as f64],
        [
            rotation.w as f64,
            rotation.x as f64,
            rotation.y as f64,
            rotation.z as f64,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parameters::BallParameters,
        scene::{
            ball::ball_spec,
            goal::{GoalDimensions, goal_spec},
        },
    };
    use types::field_dimensions::FieldDimensions;

    const ROBOT_MJCF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml");

    fn ball_object() -> MjcfObject {
        let parameters = BallParameters {
            mass: 0.450,
            joint_damping: 0.002,
            joint_friction_loss: 0.0,
            friction: [1.0, 0.005, 0.0001],
            solref: [0.08, 0.25],
            solimp: [0.9, 0.95, 0.001, 0.5, 2.0],
        };
        MjcfObject::from_factory(move || ball_spec(0.105, &parameters), "ball")
            .with_free_joint("ball_free_joint")
    }

    fn goal_object() -> MjcfObject {
        let dimensions = GoalDimensions::from(&FieldDimensions::SPL_2025);
        MjcfObject::from_factory(move || goal_spec(dimensions), "goal").with_mocap_body("goal")
    }

    #[test]
    fn coordinate_conversion_round_trips() {
        let input = Transform::from_xyz(1.0, 2.0, 3.0).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            0.2,
            -0.4,
            0.7,
        ));
        let (position, rotation) = to_mujoco(input);
        let output = from_mujoco(position, rotation);

        assert!(input.translation.abs_diff_eq(output.translation, 1e-6));
        assert!(input.rotation.abs_diff_eq(output.rotation, 1e-6));
    }

    #[test]
    fn objects_are_grounded_and_removed() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));

        let ball = app
            .world_mut()
            .spawn((ball_object().grounded(), Transform::from_xyz(1.0, 0.0, 2.0)))
            .id();
        let robot = app
            .world_mut()
            .spawn((
                MjcfObject::new(ROBOT_MJCF, "Trunk")
                    .with_free_joint("world_joint")
                    .grounded(),
                Transform::from_xyz(-1.0, 0.0, 0.0),
            ))
            .id();
        let goal = app
            .world_mut()
            .spawn((
                goal_object().grounded(),
                Transform::from_xyz(0.0, 0.0, -2.0),
            ))
            .id();
        app.update();

        let world = app.world().resource::<MujocoWorld>();
        for object in [ball, robot, goal] {
            assert!(world.minimum_object_z(object).unwrap().abs() < 1e-6);
        }
        let ball_body = MujocoBody::new(ball, "ball");
        let ball_pose = world.body_pose(&ball_body).unwrap();
        assert!((ball_pose.translation.x - 1.0).abs() < 1e-6);
        assert!((ball_pose.translation.z - 2.0).abs() < 1e-6);

        app.world_mut().despawn(ball);
        app.update();
        assert!(
            app.world()
                .resource::<MujocoWorld>()
                .body_pose(&ball_body)
                .is_none()
        );
    }

    #[test]
    fn structural_changes_preserve_existing_state() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        *app.world_mut().resource_mut::<SimulationMode>() = SimulationMode::Paused;

        let ball = app.world_mut().spawn(ball_object()).id();
        let robot = app
            .world_mut()
            .spawn(MjcfObject::new(ROBOT_MJCF, "Trunk").with_free_joint("world_joint"))
            .id();
        let goal = app.world_mut().spawn(goal_object()).id();
        app.update();

        let ball_pose =
            Transform::from_xyz(1.2, 0.8, -2.3).with_rotation(Quat::from_rotation_y(0.4));
        let goal_pose =
            Transform::from_xyz(-1.5, 0.0, 2.0).with_rotation(Quat::from_rotation_y(-0.6));
        let ball_joint = qualified_name(ball, "ball_free_joint");
        let robot_joint = qualified_name(robot, "Left_Knee_Pitch");

        {
            let mut world = app.world_mut().resource_mut::<MujocoWorld>();
            world.set_object_pose(ball, ball_pose).unwrap();
            world.set_object_pose(goal, goal_pose).unwrap();
            let data = world.data.as_mut().unwrap();
            data.joint(&ball_joint)
                .unwrap()
                .view_mut(data)
                .qvel
                .copy_from_slice(&[0.1, -0.2, 0.3, -0.4, 0.5, -0.6]);
            let mut joint = data.joint(&robot_joint).unwrap().view_mut(data);
            joint.qpos[0] = 0.7;
            joint.qvel[0] = -0.8;
            let actuator = data
                .model()
                .name_to_id(MjtObj::mjOBJ_ACTUATOR, &robot_joint)
                .unwrap();
            data.ctrl_mut()[actuator] = 3.25;
            data.set_time(12.5);
            data.forward();
        }

        let expected = state(&app, ball, robot, goal);
        let added = app.world_mut().spawn(ball_object()).id();
        app.update();
        assert_state_eq(&expected, &state(&app, ball, robot, goal));

        app.world_mut().despawn(added);
        app.update();
        assert_state_eq(&expected, &state(&app, ball, robot, goal));
    }

    struct State {
        ball_qpos: Vec<f64>,
        ball_qvel: Vec<f64>,
        robot_qpos: Vec<f64>,
        robot_qvel: Vec<f64>,
        robot_ctrl: f64,
        goal_pose: Transform,
        time: f64,
    }

    fn state(app: &App, ball: Entity, robot: Entity, goal: Entity) -> State {
        let world = app.world().resource::<MujocoWorld>();
        let data = world.data.as_ref().unwrap();
        let ball_joint = data
            .joint(&qualified_name(ball, "ball_free_joint"))
            .unwrap()
            .view(data);
        let robot_name = qualified_name(robot, "Left_Knee_Pitch");
        let robot_joint = data.joint(&robot_name).unwrap().view(data);
        let actuator = data
            .model()
            .name_to_id(MjtObj::mjOBJ_ACTUATOR, &robot_name)
            .unwrap();

        State {
            ball_qpos: ball_joint.qpos.to_vec(),
            ball_qvel: ball_joint.qvel.to_vec(),
            robot_qpos: robot_joint.qpos.to_vec(),
            robot_qvel: robot_joint.qvel.to_vec(),
            robot_ctrl: data.ctrl()[actuator],
            goal_pose: world.body_pose(&MujocoBody::new(goal, "goal")).unwrap(),
            time: data.time(),
        }
    }

    fn assert_state_eq(expected: &State, actual: &State) {
        assert_eq!(expected.ball_qpos, actual.ball_qpos);
        assert_eq!(expected.ball_qvel, actual.ball_qvel);
        assert_eq!(expected.robot_qpos, actual.robot_qpos);
        assert_eq!(expected.robot_qvel, actual.robot_qvel);
        assert_eq!(expected.robot_ctrl, actual.robot_ctrl);
        assert!(
            expected
                .goal_pose
                .translation
                .abs_diff_eq(actual.goal_pose.translation, 1e-12)
        );
        assert!(
            expected
                .goal_pose
                .rotation
                .abs_diff_eq(actual.goal_pose.rotation, 1e-12)
        );
        assert_eq!(expected.time, actual.time);
    }
}
