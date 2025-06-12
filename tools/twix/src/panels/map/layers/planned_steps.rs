use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Color32, epaint::Stroke};

use coordinate_systems::{Ground, UpcomingSupport};
use linear_algebra::{point, vector, Isometry2, Orientation3, Pose2, Pose3};
use types::{field_dimensions::FieldDimensions, step::Step, support_foot::Side};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

use super::walking::paint_sole_polygon;

pub struct PlannedSteps {
    step_plan: BufferHandle<Option<Vec<Step>>>,
    step_plan_greedy: BufferHandle<Option<Vec<Step>>>,
    step_plan_gradient: BufferHandle<Option<Vec<f32>>>,
    ground_to_upcoming_support: BufferHandle<Option<Isometry2<Ground, UpcomingSupport>>>,
    // foot_offset_left: BufferHandle<Option<Vector3<Ground>>>,
    // foot_offset_right: BufferHandle<Option<Vector3<Ground>>>,
    current_support_side: BufferHandle<Option<Option<Side>>>,
}

impl Layer<Ground> for PlannedSteps {
    const NAME: &'static str = "Planned Steps";

    fn new(nao: Arc<Nao>) -> Self {
        let step_plan = nao.subscribe_value("Control.additional_outputs.step_plan");
        let step_plan_greedy = nao.subscribe_value("Control.additional_outputs.step_plan_greedy");
        let step_plan_gradient =
            nao.subscribe_value("Control.additional_outputs.step_plan_gradient");
        let ground_to_upcoming_support =
            nao.subscribe_value("Control.additional_outputs.ground_to_upcoming_support");
        // let foot_offset_left =
        //     nao.subscribe_value("parameters.walking_engine.base.foot_offset_left");
        // let foot_offset_right =
        //     nao.subscribe_value("parameters.walking_engine.base.foot_offset_right");
        let current_support_side =
            nao.subscribe_value("Control.additional_outputs.current_support_side");

        Self {
            step_plan,
            step_plan_greedy,
            step_plan_gradient,
            ground_to_upcoming_support,
            // foot_offset_left,
            // foot_offset_right,
            current_support_side,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(step_plan) = self.step_plan.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(step_plan_greedy) = self.step_plan_greedy.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(step_plan_gradient) = self.step_plan_gradient.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(ground_to_upcoming_support) =
            self.ground_to_upcoming_support.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        // let Some(foot_offset_left) = self.foot_offset_left.get_last_value()?.flatten() else {
        //     return Ok(());
        // };
        // let Some(foot_offset_right) = self.foot_offset_right.get_last_value()?.flatten() else {
        //     return Ok(());
        // };
        let Some(current_support_side) = self.current_support_side.get_last_value()?.flatten()
        else {
            return Ok(());
        };

        paint_step_plan(
            painter,
            Color32::RED,
            ground_to_upcoming_support,
            step_plan,
            step_plan_gradient,
            current_support_side.unwrap_or(Side::Left).opposite(),
            // foot_offset_left,
            // foot_offset_right,
        );

        let dummy_gradient = vec![0.0; step_plan_greedy.len() * 3];
        paint_step_plan(
            painter,
            Color32::BLUE,
            ground_to_upcoming_support,
            step_plan_greedy,
            dummy_gradient,
            current_support_side.unwrap_or(Side::Left).opposite(),
        );

        Ok(())
    }
}

struct PlannedStep {
    pose: Pose2<Ground>,
    support_side: Side,
}

fn paint_step_plan(
    painter: &TwixPainter<Ground>,
    color: Color32,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
    step_plan: Vec<Step>,
    step_plan_gradient: Vec<f32>,
    next_support_side: Side,
    // foot_offset_left: Vector3<Ground>,
    // foot_offset_right: Vector3<Ground>,
) {
    let upcoming_support_to_ground = ground_to_upcoming_support.inverse();

    let planned_steps = step_plan.iter().scan(
        (upcoming_support_to_ground.as_pose(), next_support_side),
        |(pose, support_side), step| {
            let step_translation =
                Isometry2::<Ground, Ground>::from_parts(vector![step.forward, step.left], 0.0);
            let step_rotation =
                Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], step.turn);

            *pose = pose.as_transform() * step_rotation * step_translation.as_pose();

            let planned_step = PlannedStep {
                pose: *pose,
                support_side: *support_side,
            };

            *support_side = support_side.opposite();

            Some(planned_step)
        },
    );

    let gradients = step_plan_gradient.chunks_exact(3);
    for (PlannedStep { pose, support_side }, _gradient) in planned_steps.zip(gradients) {
        // let [df, dl, da] = gradient.try_into().unwrap();
        let offset = match support_side {
            // Side::Left => foot_offset_left,
            // Side::Right => foot_offset_right,
            Side::Left => vector!(0.0, 0.052, 0.0),
            Side::Right => vector!(0.0, -0.052, 0.0),
        };

        // painter.pose(
        //     pose,
        //     0.02,
        //     0.03,
        //     Color32::RED,
        //     Stroke::new(0.005, Color32::BLACK),
        // );

        let sole = Pose3::from_parts(
            point![pose.position().x(), pose.position().y(), 0.0] + offset,
            Orientation3::from_euler_angles(0.0, 0.0, pose.orientation().angle()),
        );

        paint_sole_polygon(painter, sole, Stroke::new(0.005, color), support_side);
        // painter.line_segment(
        //     pose.position,
        //     pose.as_transform::<Ground>() * (point![df, dl] * -0.001),
        //     Stroke::new(0.002, Color32::GREEN),
        // );
        // painter.line_segment(
        //     pose.position,
        //     pose.as_transform::<Ground>() * point![0.0, da * -0.001],
        //     Stroke::new(0.002, Color32::RED),
        // );
    }
}
