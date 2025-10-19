use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Color32, epaint::Stroke};

use coordinate_systems::{Ground, UpcomingSupport};
use linear_algebra::{point, vector, Isometry2, Orientation2, Orientation3, Pose2, Pose3, Vector3};
use step_planning::{NUM_STEPS, NUM_VARIABLES};
use types::{field_dimensions::FieldDimensions, step::Step, support_foot::Side};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

use super::walking::paint_sole_polygon;

pub struct PlannedSteps {
    direct_step: BufferHandle<Option<Step>>,
    step_plan: BufferHandle<Option<[Step; NUM_STEPS]>>,
    step_plan_greedy: BufferHandle<Option<[Step; NUM_STEPS]>>,
    step_plan_gradient: BufferHandle<Option<[f32; NUM_VARIABLES]>>,
    ground_to_upcoming_support: BufferHandle<Option<Isometry2<Ground, UpcomingSupport>>>,
    foot_offset_left: BufferHandle<Option<Vector3<Ground>>>,
    foot_offset_right: BufferHandle<Option<Vector3<Ground>>>,
    next_support_side: BufferHandle<Option<Side>>,
}

impl Layer<Ground> for PlannedSteps {
    const NAME: &'static str = "Planned Steps";

    fn new(nao: Arc<Nao>) -> Self {
        let direct_step = nao.subscribe_value("Control.additional_outputs.direct_step");
        let step_plan = nao.subscribe_value("Control.additional_outputs.step_plan");
        let step_plan_greedy = nao.subscribe_value("Control.additional_outputs.step_plan_greedy");
        let step_plan_gradient =
            nao.subscribe_value("Control.additional_outputs.step_plan_gradient");
        let ground_to_upcoming_support =
            nao.subscribe_value("Control.additional_outputs.ground_to_upcoming_support");
        let foot_offset_left =
            nao.subscribe_value("parameters.walking_engine.base.foot_offset_left");
        let foot_offset_right =
            nao.subscribe_value("parameters.walking_engine.base.foot_offset_right");
        let next_support_side = nao.subscribe_value("Control.additional_outputs.next_support_side");

        Self {
            direct_step,
            step_plan,
            step_plan_greedy,
            step_plan_gradient,
            ground_to_upcoming_support,
            foot_offset_left,
            foot_offset_right,
            next_support_side,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let direct_step = self.direct_step.get_last_value()?.flatten();
        let step_plan = self.step_plan.get_last_value()?.flatten();
        let step_plan_gradient = self.step_plan_gradient.get_last_value()?.flatten();
        let step_plan_greedy = self.step_plan_greedy.get_last_value()?.flatten();
        let Some(ground_to_upcoming_support) =
            self.ground_to_upcoming_support.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        let Some(foot_offset_left) = self.foot_offset_left.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(foot_offset_right) = self.foot_offset_right.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(next_support_side) = self.next_support_side.get_last_value()?.flatten() else {
            return Ok(());
        };

        let upcoming_support_to_ground = ground_to_upcoming_support.inverse();
        let upcoming_support_pose = upcoming_support_to_ground.as_pose();

        painter.pose(
            upcoming_support_pose,
            0.02,
            0.01,
            Color32::GRAY,
            Stroke::new(0.01, Color32::BLACK),
        );

        if let Some(direct_step) = direct_step {
            let direct_step_translation = Isometry2::<Ground, Ground>::from_parts(
                vector![direct_step.forward, direct_step.left],
                0.0,
            );
            let direct_step_rotation =
                Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], direct_step.turn);
            let direct_step_end_pose = upcoming_support_pose.as_transform()
                * direct_step_rotation
                * direct_step_translation.as_pose();

            paint_planned_step(
                painter,
                Color32::WHITE,
                direct_step_end_pose,
                next_support_side,
                foot_offset_left,
                foot_offset_right,
            );
        }

        if let (Some(step_plan), Some(step_plan_gradient)) = (step_plan, step_plan_gradient) {
            paint_step_plan(
                painter,
                Color32::RED,
                ground_to_upcoming_support,
                step_plan,
                step_plan_gradient,
                next_support_side,
                foot_offset_left,
                foot_offset_right,
            );
        }

        if let Some(step_plan_greedy) = step_plan_greedy {
            let dummy_gradient = [0.0; NUM_VARIABLES];
            paint_step_plan(
                painter,
                Color32::BLUE,
                ground_to_upcoming_support,
                step_plan_greedy,
                dummy_gradient,
                next_support_side,
                foot_offset_left,
                foot_offset_right,
            );
        }

        Ok(())
    }
}

struct PlannedStep {
    pose: Pose2<Ground>,
    support_side: Side,
}

#[expect(clippy::too_many_arguments)]
fn paint_step_plan(
    painter: &TwixPainter<Ground>,
    color: Color32,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
    step_plan: [Step; NUM_STEPS],
    step_plan_gradient: [f32; NUM_VARIABLES],
    next_support_side: Side,
    foot_offset_left: Vector3<Ground>,
    foot_offset_right: Vector3<Ground>,
) {
    let upcoming_support_to_ground = ground_to_upcoming_support.inverse();

    let upcoming_support_pose = upcoming_support_to_ground.as_pose();

    paint_planned_step(
        painter,
        color,
        upcoming_support_pose,
        next_support_side.opposite(),
        foot_offset_left,
        foot_offset_right,
    );

    let planned_steps = step_plan.iter().scan(
        (upcoming_support_pose, next_support_side),
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
    for (PlannedStep { pose, support_side }, gradient) in planned_steps.zip(gradients) {
        paint_planned_step(
            painter,
            color,
            pose,
            support_side,
            foot_offset_left,
            foot_offset_right,
        );

        let [df, dl, da] = gradient.try_into().unwrap();
        painter.line_segment(
            pose.position(),
            pose.as_transform::<Ground>() * (point![-df, -dl]),
            Stroke::new(0.002, Color32::GREEN),
        );
        painter.line_segment(
            pose.position(),
            pose.as_transform::<Ground>() * point![0.0, -da],
            Stroke::new(0.002, Color32::RED),
        );
    }
}

fn paint_planned_step(
    painter: &TwixPainter<Ground>,
    color: Color32,
    pose: Pose2<Ground>,
    support_side: Side,
    foot_offset_left: Vector3<Ground>,
    foot_offset_right: Vector3<Ground>,
) {
    let offset = match support_side {
        Side::Left => foot_offset_left,
        Side::Right => foot_offset_right,
    };

    let pose_with_offset: Pose2<Ground> = pose.as_transform::<Ground>()
        * Pose2::from_parts(offset.xy().as_point(), Orientation2::identity());

    let sole = Pose3::from_parts(
        point![
            pose_with_offset.position().x(),
            pose_with_offset.position().y(),
            0.0
        ],
        Orientation3::from_euler_angles(0.0, 0.0, pose.orientation().angle()),
    );

    painter.pose(
        pose,
        0.01,
        0.015,
        color,
        Stroke::new(0.0025, Color32::BLACK),
    );

    paint_sole_polygon(
        painter,
        sole,
        Stroke::new(0.005, color),
        support_side.opposite(),
    );
}
