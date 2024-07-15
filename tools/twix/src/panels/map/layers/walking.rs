use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Robot, Walk};
use linear_algebra::{point, vector, Isometry3, Point2, Point3, Pose2, Pose3};
use types::{
    field_dimensions::FieldDimensions, joints::body::BodyJoints, robot_kinematics::RobotKinematics,
    step_plan::Step, support_foot::Side,
};
use walking_engine::{
    mode::{
        catching::Catching, kicking::Kicking, starting::Starting, stopping::Stopping, walking, Mode,
    },
    Engine,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Walking {
    robot_to_ground: BufferHandle<Option<Isometry3<Robot, Ground>>>,
    robot_kinematics: BufferHandle<RobotKinematics>,
    walking_engine: BufferHandle<Option<Engine>>,
    last_actuated_joints: BufferHandle<Option<BodyJoints>>,
    step_plan: BufferHandle<Step>,
    center_of_mass: BufferHandle<Point3<Robot>>,
    robot_to_walk: BufferHandle<Option<Isometry3<Robot, Walk>>>,
    zero_moment_point: BufferHandle<Point2<Ground>>,
}

impl Layer<Ground> for Walking {
    const NAME: &'static str = "Walking";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_ground = nao.subscribe_value("Control.main_outputs.robot_to_ground");
        let robot_kinematics = nao.subscribe_value("Control.main_outputs.robot_kinematics");
        let walking_engine = nao.subscribe_value("Control.additional_outputs.walking.engine");
        let last_actuated_joints =
            nao.subscribe_value("Control.additional_outputs.walking.last_actuated_joints");
        let step_plan = nao.subscribe_value("Control.main_outputs.step_plan");
        let center_of_mass = nao.subscribe_value("Control.main_outputs.center_of_mass");
        let robot_to_walk = nao.subscribe_value("Control.additional_outputs.walking.robot_to_walk");
        let zero_moment_point = nao.subscribe_value("Control.main_outputs.zero_moment_point");
        Self {
            robot_to_ground,
            robot_kinematics,
            walking_engine,
            last_actuated_joints,
            step_plan,
            center_of_mass,
            robot_to_walk,
            zero_moment_point,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(robot_to_ground) = self.robot_to_ground.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(robot_kinematics) = self.robot_kinematics.get_last_value()? else {
            return Ok(());
        };
        let Some(engine) = self.walking_engine.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(last_actuated_joints) = self.last_actuated_joints.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        let Some(step_plan) = self.step_plan.get_last_value()? else {
            return Ok(());
        };
        let Some(center_of_mass) = self.center_of_mass.get_last_value()? else {
            return Ok(());
        };
        let center_of_mass_in_ground = robot_to_ground * center_of_mass;
        let Some(robot_to_walk) = self.robot_to_walk.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(zero_moment_point) = self.zero_moment_point.get_last_value()? else {
            return Ok(());
        };

        paint_actuated_feet(
            painter,
            robot_to_ground,
            last_actuated_joints,
            engine.support_side(),
            Stroke::new(0.01, Color32::BLUE),
            Stroke::new(0.003, Color32::BLUE),
        );
        paint_measured_feet(
            painter,
            robot_to_ground,
            robot_kinematics,
            engine.support_side(),
            Stroke::new(0.01, Color32::GREEN),
            Stroke::new(0.003, Color32::GREEN),
        );
        if let Engine {
            mode:
                Mode::Starting(Starting { step })
                | Mode::Walking(walking::Walking { step, .. })
                | Mode::Kicking(Kicking { step, .. })
                | Mode::Stopping(Stopping { step, .. })
                | Mode::Catching(Catching { step, .. }),
            ..
        } = engine
        {
            paint_target_feet(
                painter,
                robot_to_ground,
                last_actuated_joints,
                robot_to_walk,
                step.plan.end_feet.support_sole,
                step.plan.end_feet.swing_sole,
                step.plan.support_side,
                Stroke::new(0.004, Color32::RED),
            );
        }

        painter.circle(
            center_of_mass_in_ground.xy(),
            0.01,
            Color32::RED,
            Stroke::new(0.001, Color32::BLACK),
        );

        painter.circle(
            zero_moment_point,
            0.01,
            Color32::GRAY,
            Stroke::new(0.001, Color32::BLACK),
        );

        paint_step_plan(painter, step_plan);
        Ok(())
    }
}

fn paint_measured_feet(
    painter: &TwixPainter<Ground>,
    robot_to_ground: Isometry3<Robot, Ground>,
    robot_kinematics: RobotKinematics,
    support_side: Option<Side>,
    support_stroke: Stroke,
    swing_stroke: Stroke,
) {
    let left_sole_to_ground = robot_to_ground * robot_kinematics.left_leg.sole_to_robot;
    let right_sole_to_ground = robot_to_ground * robot_kinematics.right_leg.sole_to_robot;
    let (left_stroke, right_stroke) = match support_side {
        Some(Side::Left) => (support_stroke, swing_stroke),
        Some(Side::Right) => (swing_stroke, support_stroke),
        None => (swing_stroke, swing_stroke),
    };
    paint_sole_polygon(
        painter,
        left_sole_to_ground.as_pose(),
        left_stroke,
        Side::Left,
    );
    paint_sole_polygon(
        painter,
        right_sole_to_ground.as_pose(),
        right_stroke,
        Side::Right,
    );
}

fn paint_actuated_feet(
    painter: &TwixPainter<Ground>,
    robot_to_ground: Isometry3<Robot, Ground>,
    last_actuated_joints: BodyJoints,
    support_side: Option<Side>,
    support_stroke: Stroke,
    swing_stroke: Stroke,
) {
    let left_sole_to_ground =
        robot_to_ground * kinematics::forward::left_sole_to_robot(&last_actuated_joints.left_leg);
    let right_sole_to_ground =
        robot_to_ground * kinematics::forward::right_sole_to_robot(&last_actuated_joints.right_leg);
    let (left_stroke, right_stroke) = match support_side {
        Some(Side::Left) => (support_stroke, swing_stroke),
        Some(Side::Right) => (swing_stroke, support_stroke),
        None => (swing_stroke, swing_stroke),
    };
    paint_sole_polygon(
        painter,
        left_sole_to_ground.as_pose(),
        left_stroke,
        Side::Left,
    );
    paint_sole_polygon(
        painter,
        right_sole_to_ground.as_pose(),
        right_stroke,
        Side::Right,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_target_feet(
    painter: &TwixPainter<Ground>,
    robot_to_ground: Isometry3<Robot, Ground>,
    last_actuated_joints: BodyJoints,
    robot_to_walk: Isometry3<Robot, Walk>,
    end_support_sole: Pose3<Walk>,
    end_swing_sole: Pose3<Walk>,
    support_side: Side,
    stroke: Stroke,
) {
    struct SupportSole;
    let walk_to_robot = robot_to_walk.inverse();
    let support_sole = walk_to_robot * end_support_sole;
    let swing_sole = walk_to_robot * end_swing_sole;
    let robot_to_support_sole = support_sole.as_transform::<SupportSole>().inverse();
    let swing_as_seen_from_support = robot_to_support_sole * swing_sole;
    let actuated_support_sole_to_robot = match support_side {
        Side::Left => {
            kinematics::forward::left_sole_to_robot(&last_actuated_joints.left_leg).as_pose()
        }
        Side::Right => {
            kinematics::forward::right_sole_to_robot(&last_actuated_joints.right_leg).as_pose()
        }
    }
    .as_transform::<SupportSole>();
    paint_sole_polygon(
        painter,
        robot_to_ground * actuated_support_sole_to_robot * swing_as_seen_from_support,
        stroke,
        support_side.opposite(),
    );
}

fn paint_sole_polygon(
    painter: &TwixPainter<Ground>,
    sole: Pose3<Ground>,
    stroke: Stroke,
    side: Side,
) {
    struct Sole;

    let left_outline = [
        point![-0.05457, -0.015151, 0.0],
        point![-0.050723, -0.021379, 0.0],
        point![-0.046932, -0.025796, 0.0],
        point![-0.04262, -0.030603, 0.0],
        point![-0.037661, -0.033714, 0.0],
        point![-0.03297, -0.034351, 0.0],
        point![-0.028933, -0.033949, 0.0],
        point![-0.022292, -0.033408, 0.0],
        point![-0.015259, -0.032996, 0.0],
        point![-0.009008, -0.032717, 0.0],
        point![-0.000411, -0.032822, 0.0],
        point![0.008968, -0.033185, 0.0],
        point![0.020563, -0.034062, 0.0],
        point![0.029554, -0.035208, 0.0],
        point![0.039979, -0.03661, 0.0],
        point![0.050403, -0.038012, 0.0],
        point![0.0577, -0.038771, 0.0],
        point![0.063951, -0.038362, 0.0],
        point![0.073955, -0.03729, 0.0],
        point![0.079702, -0.03532, 0.0],
        point![0.084646, -0.033221, 0.0],
        point![0.087648, -0.031482, 0.0],
        point![0.091805, -0.027692, 0.0],
        point![0.094009, -0.024299, 0.0],
        point![0.096868, -0.018802, 0.0],
        point![0.099419, -0.01015, 0.0],
        point![0.100097, -0.001573, 0.0],
        point![0.098991, 0.008695, 0.0],
        point![0.097014, 0.016504, 0.0],
        point![0.093996, 0.02418, 0.0],
        point![0.090463, 0.02951, 0.0],
        point![0.084545, 0.0361, 0.0],
        point![0.079895, 0.039545, 0.0],
        point![0.074154, 0.042654, 0.0],
        point![0.065678, 0.046145, 0.0],
        point![0.057207, 0.047683, 0.0],
        point![0.049911, 0.048183, 0.0],
        point![0.039758, 0.045938, 0.0],
        point![0.029217, 0.042781, 0.0],
        point![0.020366, 0.04054, 0.0],
        point![0.00956, 0.038815, 0.0],
        point![0.000183, 0.038266, 0.0],
        point![-0.008417, 0.039543, 0.0],
        point![-0.015589, 0.042387, 0.0],
        point![-0.021987, 0.047578, 0.0],
        point![-0.027076, 0.050689, 0.0],
        point![-0.031248, 0.051719, 0.0],
        point![-0.03593, 0.049621, 0.0],
        point![-0.040999, 0.045959, 0.0],
        point![-0.045156, 0.042039, 0.0],
        point![-0.04905, 0.037599, 0.0],
        point![-0.054657, 0.029814, 0.0],
        point![-0.05457, -0.015151, 0.0],
        point![-0.050723, -0.021379, 0.0],
    ];

    let sole_to_ground = sole.as_transform::<Sole>();
    painter.polygon(
        left_outline.into_iter().map(|point| {
            match side {
                Side::Left => sole_to_ground * point,
                Side::Right => sole_to_ground * point![point.x(), -point.y(), point.z()],
            }
            .xy()
        }),
        stroke,
    );
}

fn paint_step_plan(painter: &TwixPainter<Ground>, step_plan: Step) {
    painter.pose(
        Pose2::default(),
        0.02,
        0.03,
        Color32::TRANSPARENT,
        Stroke::new(0.005, Color32::BLACK),
    );
    painter.pose(
        Pose2::new(vector![step_plan.forward, step_plan.left], step_plan.turn),
        0.02,
        0.03,
        Color32::RED,
        Stroke::new(0.005, Color32::BLACK),
    );
}
