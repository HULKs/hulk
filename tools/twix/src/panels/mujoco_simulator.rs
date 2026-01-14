use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy_mujoco::{MujocoVisualizerPlugin, TrunkComponent};
use eframe::egui::{Response, Ui, Widget};
use egui_bevy::BevyWidget;
use linear_algebra::Isometry3;
use types::robot_kinematics::RobotKinematics;

use crate::{
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};

pub struct MujocoSimulatorPanel {
    widget: BevyWidget,
    kinematics: BufferHandle<RobotKinematics>,
    camera_to_world: BufferHandle<Isometry3<Camera, World>>,
}

impl<'a> Panel<'a> for MujocoSimulatorPanel {
    const NAME: &'static str = "Mujoco Simulator";

    fn new(context: PanelCreationContext) -> Self {
        let mut widget = BevyWidget::new(context.wgpu_state.clone());
        widget
            .bevy_app
            .add_plugins(MujocoVisualizerPlugin::new(context.egui_context.clone()))
            .init_resource::<KinematicsResource>()
            .init_resource::<FakeCameraResource>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_systems(Update, draw_gizmos);
        widget.bevy_app.finish();
        widget.bevy_app.cleanup();

        let kinematics = context
            .nao
            .subscribe_value("Control.main_outputs.robot_kinematics");
        let camera_to_world = context
            .nao
            .subscribe_value("Control.main_outputs.camera_to_world");

        Self {
            widget,
            kinematics,
            camera_to_world,
        }
    }
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Ok(Some(value)) = self.kinematics.get_last_value() {
            self.widget
                .bevy_app
                .world_mut()
                .insert_resource(KinematicsResource { value });
        };
        if let Ok(Some(value)) = self.camera_to_world.get_last_value() {
            self.widget
                .bevy_app
                .world_mut()
                .insert_resource(FakeCameraResource { value });
        };
        self.widget.ui(ui)
    }
}

#[derive(Resource, Default)]
struct KinematicsResource {
    value: RobotKinematics,
}
#[derive(Resource, Default)]
struct FakeCameraResource {
    value: Isometry3<Camera, World>,
}

fn draw_gizmos(
    robot: Single<(&GlobalTransform, &TrunkComponent)>,
    kinematics: Res<KinematicsResource>,
    camera: Res<FakeCameraResource>,
    mut gizmos: Gizmos,
) {
    let (translation, rotation) = camera.value.inner.into();
    gizmos.axes(
        Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2))
            * Transform::from_isometry(Isometry3d::new(translation, rotation)),
        0.1,
    );
    gizmos.axes(Transform::IDENTITY, 1.0);
    let mut draw = |pose: nalgebra::Isometry3<f32>| {
        let (translation, rotation) =
            (kinematics.value.torso.torso_to_robot.inner.inverse() * pose).into();
        gizmos.axes(
            *robot.0 * Transform::from_isometry(Isometry3d::new(translation, rotation)),
            0.1,
        );
    };
    draw(kinematics.value.head.neck_to_robot.inner);
    draw(kinematics.value.head.head_to_robot.inner);
    draw(kinematics.value.left_arm.inner_shoulder_to_robot.inner);
    draw(kinematics.value.left_arm.outer_shoulder_to_robot.inner);
    draw(kinematics.value.left_arm.upper_arm_to_robot.inner);
    draw(kinematics.value.left_arm.forearm_to_robot.inner);
    draw(kinematics.value.right_arm.inner_shoulder_to_robot.inner);
    draw(kinematics.value.right_arm.outer_shoulder_to_robot.inner);
    draw(kinematics.value.right_arm.upper_arm_to_robot.inner);
    draw(kinematics.value.right_arm.forearm_to_robot.inner);
    draw(kinematics.value.left_leg.pelvis_to_robot.inner);
    draw(kinematics.value.left_leg.hip_to_robot.inner);
    draw(kinematics.value.left_leg.thigh_to_robot.inner);
    draw(kinematics.value.left_leg.tibia_to_robot.inner);
    draw(kinematics.value.left_leg.ankle_to_robot.inner);
    draw(kinematics.value.left_leg.foot_to_robot.inner);
    draw(kinematics.value.right_leg.pelvis_to_robot.inner);
    draw(kinematics.value.right_leg.hip_to_robot.inner);
    draw(kinematics.value.right_leg.thigh_to_robot.inner);
    draw(kinematics.value.right_leg.tibia_to_robot.inner);
    draw(kinematics.value.right_leg.ankle_to_robot.inner);
    draw(kinematics.value.right_leg.foot_to_robot.inner);
}
