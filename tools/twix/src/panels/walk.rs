use std::sync::Arc;

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};
use coordinate_systems::{Ground, Robot, Walk};
use eframe::egui::{CentralPanel, Color32, Response, Stroke, TopBottomPanel, Ui, Widget};
use egui_plot::{MarkerShape, Plot, PlotPoint, PlotPoints, PlotUi, Points, Polygon};
use linear_algebra::{Isometry3, Point2, Point3, Pose3};
use serde_json::{json, Value};
use types::{
    joints::Joints,
    motor_commands::MotorCommands,
    robot_dimensions::{self},
    support_foot::Side,
};
use walking_engine::{
    feet::Feet,
    mode::{
        catching::Catching, kicking::Kicking, starting::Starting, stopping::Stopping,
        walking::Walking, Mode,
    },
    step_state::StepState,
    Engine,
};

pub struct WalkPanel {
    walking_engine: BufferHandle<Option<Engine>>,
    robot_to_walk: BufferHandle<Option<Isometry3<Robot, Walk>>>,
    last_actuated_commands: BufferHandle<Option<MotorCommands<Joints<f32>>>>,
    robot_to_ground: BufferHandle<Option<Isometry3<Robot, Ground>>>,
    zero_moment_point: BufferHandle<Point2<Ground>>,
}

impl Panel for WalkPanel {
    const NAME: &'static str = "Walk";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let walking_engine = nao.subscribe_value("Control.additional_outputs.walking.engine");
        let robot_to_walk = nao.subscribe_value("Control.additional_outputs.walking.robot_to_walk");
        let last_actuated_commands =
            nao.subscribe_value("Control.additional_outputs.actuated_motor_commands");
        let robot_to_ground = nao.subscribe_value("Control.main_outputs.robot_to_ground");
        let zero_moment_point = nao.subscribe_value("Control.main_outputs.zero_moment_point");

        Self {
            walking_engine,
            robot_to_walk,
            last_actuated_commands,
            robot_to_ground,
            zero_moment_point,
        }
    }
    fn save(&self) -> Value {
        json!({})
    }
}

impl Widget for &mut WalkPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let engine = match self.walking_engine.get_last_value() {
            Ok(Some(Some(engine))) => engine,
            Ok(_) => return ui.label("no walking engine"),
            Err(error) => {
                return ui.colored_label(Color32::RED, format!("Error (engine): {}", error));
            }
        };

        let robot_to_walk = match self.robot_to_walk.get_last_value() {
            Ok(Some(Some(robot_to_walk))) => robot_to_walk,
            Ok(_) => return ui.label("no robot to walk"),
            Err(error) => {
                return ui.colored_label(Color32::RED, format!("Error (robot_to_walk): {}", error));
            }
        };

        let last_actuated_joints = match self.last_actuated_commands.get_last_value() {
            Ok(Some(Some(commands))) => commands.positions,
            Ok(_) => return ui.label("no last actuated commands"),
            Err(error) => {
                return ui.colored_label(
                    Color32::RED,
                    format!("Error (last_actuated_joints): {}", error),
                );
            }
        };

        let robot_to_ground = match self.robot_to_ground.get_last_value() {
            Ok(Some(Some(robot_to_ground))) => robot_to_ground,
            Ok(_) => return ui.label("no robot to ground"),
            Err(error) => {
                return ui
                    .colored_label(Color32::RED, format!("Error (robot_to_ground): {}", error));
            }
        };

        let zero_moment_point = match self.zero_moment_point.get_last_value() {
            Ok(Some(zero_moment_point)) => zero_moment_point,
            Ok(_) => return ui.label("no zero moment point"),
            Err(error) => {
                return ui.colored_label(
                    Color32::RED,
                    format!("Error (zero moment point): {}", error),
                );
            }
        };

        let zero_moment_point_in_walk =
            robot_to_walk * robot_to_ground.inverse() * zero_moment_point.extend(0.0);

        let central_panel = CentralPanel::default().show_inside(ui, |ui| {
            draw_top_down_plot(
                ui,
                &engine,
                robot_to_walk,
                last_actuated_joints,
                zero_moment_point_in_walk,
            );
        });

        central_panel.response
    }
}

fn step_plan(engine: &Engine) -> Option<&StepState> {
    match &engine.mode {
        Mode::Starting(Starting { step }) => Some(step),
        Mode::Walking(Walking { step, .. }) => Some(step),
        Mode::Kicking(Kicking { step, .. }) => Some(step),
        Mode::Catching(Catching { step }) => Some(step),
        Mode::Stopping(Stopping { step, .. }) => Some(step),
        _ => None,
    }
}

fn draw_top_down_plot(
    ui: &mut Ui,
    engine: &Engine,
    robot_to_walk: Isometry3<Robot, Walk>,
    last_actuated_joints: Joints,
    zero_moment_point: Point3<Walk>,
) -> Option<Response> {
    let step = step_plan(engine)?;
    let response = Plot::new(ui.next_auto_id().with("Walk Top Down Plot"))
        .data_aspect(1.0)
        .show(ui, |ui| {
            let support_side = step.plan.support_side;

            let start_feet = step.plan.start_feet;
            let start_feet_color = Color32::ORANGE;
            plot_feet(ui, support_side, start_feet, start_feet_color);

            let end_feet = step.plan.end_feet;
            let end_feet_color = Color32::RED;
            plot_feet(ui, support_side, end_feet, end_feet_color);

            let current_feet =
                Feet::from_joints(robot_to_walk, &last_actuated_joints.body(), support_side);
            let current_feet_color = Color32::BLUE;
            plot_feet(ui, support_side, current_feet, current_feet_color);
            ui.points(
                Points::new(PlotPoints::Owned(vec![PlotPoint::new(
                    zero_moment_point.x(),
                    zero_moment_point.y(),
                )]))
                .radius(5.0)
                .shape(MarkerShape::Asterisk),
            );
        });
    Some(response.response)
}

fn draw_side_plot(
    ui: &mut Ui,
    engine: &Engine,
    robot_to_walk: Isometry3<Robot, Walk>,
    last_actuated_joints: Joints,
    zero_moment_point: Point3<Walk>,
) -> Option<Response> {
    let step = step_plan(engine)?;
    let response = Plot::new(ui.next_auto_id().with("Walk Top Down Plot"))
        .data_aspect(1.0)
        .show(ui, |ui| {
            let support_side = step.plan.support_side;

            let start_feet = step.plan.start_feet;
            let start_feet_color = Color32::ORANGE;
            plot_feet_side(ui, support_side, start_feet, start_feet_color);

            let end_feet = step.plan.end_feet;
            let end_feet_color = Color32::RED;
            plot_feet_side(ui, support_side, end_feet, end_feet_color);

            let current_feet =
                Feet::from_joints(robot_to_walk, &last_actuated_joints.body(), support_side);
            let current_feet_color = Color32::BLUE;
            plot_feet_side(ui, support_side, current_feet, current_feet_color);
            ui.points(
                Points::new(PlotPoints::Owned(vec![PlotPoint::new(
                    zero_moment_point.x(),
                    zero_moment_point.y(),
                )]))
                .radius(5.0)
                .shape(MarkerShape::Asterisk),
            );
        });
    Some(response.response)
}

fn plot_feet(ui: &mut PlotUi, support_side: Side, feet: Feet, color: Color32) {
    match support_side {
        Side::Left => {
            plot_sole_outline(ui, support_side, Side::Left, feet.support_sole, color);
            plot_sole_outline(ui, support_side, Side::Right, feet.swing_sole, color);
        }
        Side::Right => {
            plot_sole_outline(ui, support_side, Side::Right, feet.support_sole, color);
            plot_sole_outline(ui, support_side, Side::Left, feet.swing_sole, color);
        }
    }
}

fn plot_sole_outline(
    ui: &mut PlotUi,
    support_side: Side,
    side: Side,
    pose: Pose3<Walk>,
    color: Color32,
) {
    let outline: Vec<_> = match side {
        Side::Left => {
            let transform = pose.as_transform();
            robot_dimensions::transform_left_sole_outline(transform)
                .map(|point| point.xy())
                .collect()
        }
        Side::Right => {
            let transform = pose.as_transform();
            robot_dimensions::transform_right_sole_outline(transform)
                .map(|point| point.xy())
                .collect()
        }
    };
    let plot_points = outline
        .into_iter()
        .map(|point| PlotPoint::new(point.x() as f64, point.y() as f64))
        .collect::<Vec<_>>();

    let stroke_width = if support_side == side { 5.0 } else { 1.0 };

    ui.polygon(
        Polygon::new(PlotPoints::Owned(plot_points))
            .stroke(Stroke::new(stroke_width, color))
            .fill_color(color.gamma_multiply(0.2)),
    );
}

fn plot_feet_side(ui: &mut PlotUi, support_side: Side, feet: Feet, color: Color32) {
    match support_side {
        Side::Left => {
            plot_sole_outline_side(ui, support_side, Side::Left, feet.support_sole, color);
            plot_sole_outline_side(ui, support_side, Side::Right, feet.swing_sole, color);
        }
        Side::Right => {
            plot_sole_outline_side(ui, support_side, Side::Right, feet.support_sole, color);
            plot_sole_outline_side(ui, support_side, Side::Left, feet.swing_sole, color);
        }
    }
}

fn plot_sole_outline_side(
    ui: &mut PlotUi,
    support_side: Side,
    side: Side,
    pose: Pose3<Walk>,
    color: Color32,
) {
    let outline: Vec<_> = match side {
        Side::Left => {
            let transform = pose.as_transform();
            robot_dimensions::transform_left_sole_outline(transform)
                .map(|point| point.xz())
                .collect()
        }
        Side::Right => {
            let transform = pose.as_transform();
            robot_dimensions::transform_right_sole_outline(transform)
                .map(|point| point.xz())
                .collect()
        }
    };
    let plot_points = outline
        .into_iter()
        .map(|point| PlotPoint::new(point.x() as f64, point.y() as f64))
        .collect::<Vec<_>>();

    let stroke_width = if support_side == side { 5.0 } else { 1.0 };

    ui.polygon(
        Polygon::new(PlotPoints::Owned(plot_points))
            .stroke(Stroke::new(stroke_width, color))
            .fill_color(color.gamma_multiply(0.2)),
    );
}
