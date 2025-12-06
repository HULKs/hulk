use bevy::prelude::*;
use bevy_mujoco::{MujocoVisualizerPlugin, TrunkComponent};
use eframe::egui::{Response, Ui, Widget};
use egui_bevy::BevyWidget;
use nalgebra::Isometry3;
use types::robot_kinematics::RobotKinematics;

use crate::{
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};

pub struct MujocoSimulatorPanel {
    widget: BevyWidget,
    kinematics: BufferHandle<RobotKinematics>,
}

impl<'a> Panel<'a> for MujocoSimulatorPanel {
    const NAME: &'static str = "Mujoco Simulator";

    fn new(context: PanelCreationContext) -> Self {
        let mut widget = BevyWidget::new(context.wgpu_state.clone());
        widget
            .bevy_app
            .add_plugins(MujocoVisualizerPlugin::new(context.egui_context.clone()))
            .init_resource::<KinematicsResource>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_systems(Update, draw_gizmos);
        widget.bevy_app.finish();
        widget.bevy_app.cleanup();

        let kinematics = context
            .nao
            .subscribe_value("Control.main_outputs.robot_kinematics");
        Self { widget, kinematics }
    }
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Ok(Some(kinematics)) = self.kinematics.get_last_value() {
            self.widget
                .bevy_app
                .world_mut()
                .insert_resource(KinematicsResource { value: kinematics });
        };
        self.widget.ui(ui)
    }
}

#[derive(Resource, Default)]
struct KinematicsResource {
    value: RobotKinematics,
}

fn draw_gizmos(
    robot: Single<(&GlobalTransform, &TrunkComponent)>,
    kinematics: Res<KinematicsResource>,
    mut gizmos: Gizmos,
) {
    let mut draw = |pose: Isometry3<f32>| {
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
