use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Color32, epaint::Stroke};

use coordinate_systems::{Ground, UpcomingSupport};
use linear_algebra::{point, vector, Isometry, Isometry2};
use step_planning::step_plan::StepPlan;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct PlannedSteps {
    step_plan: BufferHandle<Option<Vec<f32>>>,
    step_plan_gradient: BufferHandle<Option<Vec<f32>>>,
    ground_to_upcoming_support: BufferHandle<Option<Isometry2<Ground, UpcomingSupport>>>,
}

impl Layer<Ground> for PlannedSteps {
    const NAME: &'static str = "Planned Steps";

    fn new(nao: Arc<Nao>) -> Self {
        let step_plan = nao.subscribe_value("Control.additional_outputs.step_plan");
        let step_plan_gradient =
            nao.subscribe_value("Control.additional_outputs.step_plan_gradient");
        let ground_to_upcoming_support =
            nao.subscribe_value("Control.additional_outputs.ground_to_upcoming_support");

        Self {
            step_plan,
            step_plan_gradient,
            ground_to_upcoming_support,
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
        let Some(step_plan_gradient) = self.step_plan_gradient.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(ground_to_upcoming_support) =
            self.ground_to_upcoming_support.get_last_value()?.flatten()
        else {
            return Ok(());
        };

        paint_step_plan(
            painter,
            ground_to_upcoming_support,
            step_plan,
            step_plan_gradient,
        );

        Ok(())
    }
}

fn paint_step_plan(
    painter: &TwixPainter<Ground>,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
    step_plan: Vec<f32>,
    step_plan_gradient: Vec<f32>,
) {
    let step_plan = StepPlan::from(step_plan.as_slice());
    let upcoming_support_to_ground = ground_to_upcoming_support.inverse();

    let step_end_poses =
        step_plan
            .steps()
            .scan(upcoming_support_to_ground.as_pose(), |pose, step| {
                let step_isometry = Isometry::<Ground, Ground, 2, _, _>::from_parts(
                    vector![step.forward, step.left],
                    step.turn,
                );

                *pose = step_isometry * *pose;

                Some(*pose)
            });

    let gradients = step_plan_gradient.chunks_exact(3);
    for (pose, gradient) in step_end_poses.zip(gradients) {
        let [df, dl, da] = gradient.try_into().unwrap();

        painter.pose(
            pose,
            0.02,
            0.03,
            Color32::RED,
            Stroke::new(0.005, Color32::BLACK),
        );
        painter.line_segment(
            pose.position(),
            pose.as_transform::<Ground>() * (point![df, dl] * -0.001),
            Stroke::new(0.002, Color32::GREEN),
        );
        painter.line_segment(
            pose.position(),
            pose.as_transform::<Ground>() * point![0.0, da * -0.001],
            Stroke::new(0.002, Color32::RED),
        );
    }
}
