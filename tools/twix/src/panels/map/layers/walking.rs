use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Robot, UpcomingSupport, Walk};
use linear_algebra::{point, Isometry2, Isometry3, Point2, Point3, Pose2, Pose3, Vector2};
use types::{
    field_dimensions::FieldDimensions, joints::body::BodyJoints, motor_commands::MotorCommands,
    robot_kinematics::RobotKinematics, step::Step, support_foot::Side,
};
use walking_engine::{
    feet::Feet,
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
    ground_to_upcoming_support: BufferHandle<Option<Isometry2<Ground, UpcomingSupport>>>,
    robot_kinematics: BufferHandle<RobotKinematics>,
    walking_engine: BufferHandle<Option<Engine>>,
    walk_motor_commands: BufferHandle<MotorCommands<BodyJoints<f32>>>,
    planned_step: BufferHandle<Step>,
    center_of_mass: BufferHandle<Point3<Robot>>,
    robot_to_walk: BufferHandle<Option<Isometry3<Robot, Walk>>>,
    zero_moment_point: BufferHandle<Point2<Ground>>,
}

impl Layer<Ground> for Walking {
    const NAME: &'static str = "Walking";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_ground = nao.subscribe_value("Control.main_outputs.robot_to_ground");
        let ground_to_upcoming_support =
            nao.subscribe_value("Control.additional_outputs.ground_to_upcoming_support");
        let robot_kinematics = nao.subscribe_value("Control.main_outputs.robot_kinematics");
        let walking_engine = nao.subscribe_value("Control.additional_outputs.walking.engine");
        let walk_motor_commands = nao.subscribe_value("Control.main_outputs.walk_motor_commands");
        let planned_step = nao.subscribe_value("Control.main_outputs.planned_step");
        let center_of_mass = nao.subscribe_value("Control.main_outputs.center_of_mass");
        let robot_to_walk = nao.subscribe_value("Control.additional_outputs.walking.robot_to_walk");
        let zero_moment_point = nao.subscribe_value("Control.main_outputs.zero_moment_point");

        Self {
            robot_to_ground,
            ground_to_upcoming_support,
            robot_kinematics,
            walking_engine,
            walk_motor_commands,
            planned_step,
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
        let Some(ground_to_upcoming_support) =
            self.ground_to_upcoming_support.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        let Some(robot_kinematics) = self.robot_kinematics.get_last_value()? else {
            return Ok(());
        };
        let Some(engine) = self.walking_engine.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(actuated_motor_commands) = self.walk_motor_commands.get_last_value()? else {
            return Ok(());
        };
        let Some(planned_step) = self.planned_step.get_last_value()? else {
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

        paint_walk_frame(painter, robot_to_ground, robot_to_walk);

        paint_actuated_feet(
            painter,
            robot_to_ground,
            actuated_motor_commands.positions,
            engine.mode.support_side(),
            Stroke::new(0.01, Color32::BLUE),
            Stroke::new(0.003, Color32::BLUE),
        );
        paint_measured_feet(
            painter,
            robot_to_ground,
            robot_kinematics,
            engine.mode.support_side(),
            Stroke::new(0.01, Color32::GREEN),
            Stroke::new(0.003, Color32::GREEN),
        );
        if let Engine {
            mode:
                Mode::Starting(Starting { step })
                | Mode::Walking(walking::Walking { step, .. })
                | Mode::Kicking(Kicking { step, .. })
                | Mode::Catching(Catching { step })
                | Mode::Stopping(Stopping { step, .. }),
            ..
        } = engine
        {
            paint_target_feet(
                painter,
                robot_to_ground,
                actuated_motor_commands.positions,
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

        paint_planned_step(painter, planned_step, ground_to_upcoming_support);
        Ok(())
    }
}

fn paint_walk_frame(
    painter: &TwixPainter<Ground>,
    robot_to_ground: Isometry3<Robot, Ground>,
    robot_to_walk: Isometry3<Robot, Walk>,
) {
    let walk_to_robot = robot_to_walk.inverse();
    let walk_to_ground = robot_to_ground * walk_to_robot;

    let origin = (walk_to_ground * Point3::origin()).xy();
    painter.line_segment(
        origin - Vector2::x_axis() * 0.5,
        origin + Vector2::x_axis() * 0.5,
        Stroke::new(0.005, Color32::BLACK),
    );
    painter.line_segment(
        origin - Vector2::y_axis() * 0.5,
        origin + Vector2::y_axis() * 0.5,
        Stroke::new(0.005, Color32::BLACK),
    );
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
    let walk_to_robot = robot_to_walk.inverse();
    let current_feet = Feet::from_joints(robot_to_walk, &last_actuated_joints, support_side);

    struct SupportSole;
    let upcoming_walk_to_support_sole = end_support_sole.as_transform::<SupportSole>().inverse();
    // the red swing foot
    let target_swing_sole =
        current_feet.support_sole.as_transform() * upcoming_walk_to_support_sole * end_swing_sole;

    paint_sole_polygon(
        painter,
        robot_to_ground * walk_to_robot * target_swing_sole,
        stroke,
        support_side.opposite(),
    );

    paint_sole_polygon(
        painter,
        robot_to_ground * walk_to_robot * end_swing_sole,
        Stroke::new(stroke.width, Color32::PURPLE),
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

fn paint_planned_step(
    painter: &TwixPainter<Ground>,
    planned_step: Step,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
) {
    painter.pose(
        Pose2::default(),
        0.02,
        0.03,
        Color32::TRANSPARENT,
        Stroke::new(0.005, Color32::BLACK),
    );
    painter.pose(
        ground_to_upcoming_support.inverse()
            * Pose2::new(
                point![planned_step.forward, planned_step.left],
                planned_step.turn,
            ),
        0.02,
        0.03,
        Color32::RED,
        Stroke::new(0.005, Color32::BLACK),
    );
}
