use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::{
    Result,
    eyre::{Report, WrapErr},
};
use coordinate_systems::{Field, World};
use eframe::{
    App, Frame, NativeOptions,
    egui::{
        Align2, CentralPanel, CollapsingHeader, ComboBox, Context, FontId, Label, RichText,
        ScrollArea, SidePanel, Slider, TextEdit, TopBottomPanel, Ui, WidgetText,
    },
    epaint::{Color32, Stroke},
    run_native,
};
use egui_dock::{DockArea, DockState, Node, Split, TabViewer};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Orientation2, Pose2, point, vector};
use serde_json::{Value, json};
use twix::{
    behavior_tree::BehaviorTreeVisualizer,
    twix_painter::{Orientation, TwixPainter},
    zoom_and_pan::ZoomAndPanTransform,
};
use types::{field_dimensions::FieldDimensions, motion_command::MotionCommand};

use crate::behavior_tree_simulator::{SimulatorFailure, TimelineFrame};

#[derive(Debug)]
pub struct TimelineViewerData {
    pub field_dimensions: FieldDimensions,
    pub frames: Vec<TimelineFrame>,
    pub failures: Vec<SimulatorFailure>,
}

pub fn show_timeline_viewer(data: TimelineViewerData) -> Result<()> {
    run_native(
        "Behavior Tree Simulator",
        NativeOptions::default(),
        Box::new(move |_creation_context| Ok(Box::new(TimelineViewerApp::new(data)))),
    )
    .map_err(|error| Report::msg(error.to_string()))
    .wrap_err("failed to run behavior tree simulator viewer")
}

struct TimelineViewerApp {
    data: TimelineViewerData,
    selected_frame: usize,
    is_playing: bool,
    playback_speed: f32,
    last_playback_update: Instant,
    playback_time_accumulator: f64,
    inspector_filter: String,
    inspector_cache_frame: Option<usize>,
    inspector_cache: Option<Value>,
    zoom_and_pan: ZoomAndPanTransform,
    selected_trace_robot: Option<PlayerNumber>,
    behavior_tree_visualizer: BehaviorTreeVisualizer,
    dock_state: DockState<TimelineViewerTab>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimelineViewerTab {
    Field,
    BehaviorTree,
}

impl TimelineViewerApp {
    fn new(data: TimelineViewerData) -> Self {
        let mut dock_state = DockState::new(vec![TimelineViewerTab::Field]);
        dock_state.split(
            (0.into(), 0.into()),
            Split::Below,
            0.42,
            Node::leaf(TimelineViewerTab::BehaviorTree),
        );

        Self {
            data,
            selected_frame: 0,
            is_playing: false,
            playback_speed: 1.0,
            last_playback_update: Instant::now(),
            playback_time_accumulator: 0.0,
            inspector_filter: String::new(),
            inspector_cache_frame: None,
            inspector_cache: None,
            zoom_and_pan: ZoomAndPanTransform::default(),
            selected_trace_robot: None,
            behavior_tree_visualizer: BehaviorTreeVisualizer::default(),
            dock_state,
        }
    }

    fn selected_frame(&self) -> Option<&TimelineFrame> {
        self.data.frames.get(self.selected_frame)
    }

    fn clamp_selected_frame(&mut self) {
        if self.data.frames.is_empty() {
            self.selected_frame = 0;
        } else {
            self.selected_frame = self.selected_frame.min(self.data.frames.len() - 1);
        }
    }

    fn advance_playback(&mut self, context: &Context) {
        if !self.is_playing {
            self.last_playback_update = Instant::now();
            return;
        }

        if self.data.frames.len() <= 1 || self.selected_frame + 1 >= self.data.frames.len() {
            self.is_playing = false;
            self.playback_time_accumulator = 0.0;
            return;
        }

        let now = Instant::now();
        self.playback_time_accumulator +=
            now.duration_since(self.last_playback_update).as_secs_f64()
                * f64::from(self.playback_speed);
        self.last_playback_update = now;

        while self.selected_frame + 1 < self.data.frames.len() {
            let frame_duration = frame_duration_seconds(
                &self.data.frames[self.selected_frame],
                &self.data.frames[self.selected_frame + 1],
            );
            if self.playback_time_accumulator < frame_duration {
                break;
            }
            self.playback_time_accumulator -= frame_duration;
            self.selected_frame += 1;
        }

        if self.selected_frame + 1 >= self.data.frames.len() {
            self.is_playing = false;
            self.playback_time_accumulator = 0.0;
        } else {
            context.request_repaint();
        }
    }

    fn show_top_panel(&self, context: &Context) {
        TopBottomPanel::top("timeline_viewer_top_panel").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Behavior Tree Simulator");
                ui.separator();
                if self.data.failures.is_empty() {
                    ui.colored_label(Color32::LIGHT_GREEN, "passed");
                } else {
                    ui.colored_label(Color32::LIGHT_RED, "failed");
                }
                ui.label(format!("frames: {}", self.data.frames.len()));
                if let Some(frame) = self.selected_frame() {
                    ui.label(format!("time: {}", format_time(frame.now)));
                }
            });
        });
    }

    fn show_side_panel(&mut self, context: &Context) {
        SidePanel::right("timeline_viewer_side_panel")
            .resizable(true)
            .default_width(360.0)
            .max_width(1000.0)
            .show(context, |ui| {
                ui.heading("Frame");
                ui.label(format!("index: {}", self.selected_frame));

                if let Some(frame) = self.selected_frame() {
                    ui.separator();
                    ui.heading("Robots");
                    for (player_number, robot_frame) in &frame.robot_frames {
                        ui.label(format!(
                            "robot {player_number}: {}",
                            motion_name(&robot_frame.motion_command)
                        ));
                    }

                    ui.separator();
                    ui.heading("Violations");
                    if frame.invariant_violations.is_empty() {
                        ui.label("none");
                    } else {
                        for violation in &frame.invariant_violations {
                            ui.colored_label(Color32::LIGHT_RED, violation.to_string());
                        }
                    }
                }

                if !self.data.failures.is_empty() {
                    ui.separator();
                    ui.heading("Scenario Failures");
                    for failure in &self.data.failures {
                        ui.label(failure.to_string());
                    }
                }

                ui.separator();
                self.show_state_inspector(ui);
                ui.allocate_space(ui.available_size());
            });
    }

    fn show_state_inspector(&mut self, ui: &mut Ui) {
        ui.heading("State Inspector");
        ui.horizontal(|ui| {
            ui.label("filter");
            if ui.button("clear").clicked() {
                self.inspector_filter.clear();
            }
        });
        ui.add_sized(
            [ui.available_width(), ui.spacing().interact_size.y],
            TextEdit::singleline(&mut self.inspector_filter).hint_text("path or value"),
        );

        let filter = self.inspector_filter.trim().to_ascii_lowercase();
        let Some(value) = self.selected_inspection_value() else {
            ui.label("no frame selected");
            return;
        };

        if !json_value_matches(value, "frame", &filter) {
            ui.label("no matches");
            return;
        }

        ScrollArea::vertical()
            .id_salt("timeline_viewer_state_inspector")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                show_json_value(ui, "frame", value, "frame", &filter);
            });
    }

    fn selected_inspection_value(&mut self) -> Option<&Value> {
        if self.inspector_cache_frame != Some(self.selected_frame) {
            self.inspector_cache_frame = Some(self.selected_frame);
            self.inspector_cache = self
                .data
                .frames
                .get(self.selected_frame)
                .map(frame_inspection_value);
        }

        self.inspector_cache.as_ref()
    }

    fn show_timeline_scrubber(&mut self, context: &Context) {
        TopBottomPanel::bottom("timeline_viewer_scrubber").show(context, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if self.is_playing { "pause" } else { "play" })
                    .clicked()
                {
                    self.is_playing = !self.is_playing;
                    self.last_playback_update = Instant::now();
                    self.playback_time_accumulator = 0.0;
                }

                ui.add(
                    Slider::new(&mut self.playback_speed, 0.1..=10.0)
                        .logarithmic(true)
                        .text("speed")
                        .suffix("x"),
                );

                if ui.button("previous").clicked() {
                    self.selected_frame = self.selected_frame.saturating_sub(1);
                    self.playback_time_accumulator = 0.0;
                }

                if ui.button("next").clicked() && self.selected_frame + 1 < self.data.frames.len() {
                    self.selected_frame += 1;
                    self.playback_time_accumulator = 0.0;
                }

                if self.data.frames.is_empty() {
                    ui.label("no frames recorded");
                } else {
                    let max_frame = self.data.frames.len() - 1;
                    let slider_response = ui.add(
                        Slider::new(&mut self.selected_frame, 0..=max_frame)
                            .text("frame")
                            .show_value(true),
                    );
                    if slider_response.changed() {
                        self.playback_time_accumulator = 0.0;
                    }
                }
            });
        });
    }

    fn show_dock_area(&mut self, context: &Context) {
        CentralPanel::default().show(context, |ui| {
            let mut tab_viewer = TimelineDockViewer {
                data: &self.data,
                selected_frame: self.selected_frame,
                zoom_and_pan: &mut self.zoom_and_pan,
                selected_trace_robot: &mut self.selected_trace_robot,
                behavior_tree_visualizer: &mut self.behavior_tree_visualizer,
            };
            DockArea::new(&mut self.dock_state).show_inside(ui, &mut tab_viewer);
        });
    }
}

impl App for TimelineViewerApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        self.clamp_selected_frame();
        self.advance_playback(context);
        self.show_top_panel(context);
        self.show_side_panel(context);
        self.show_timeline_scrubber(context);
        self.show_dock_area(context);
    }
}

struct TimelineDockViewer<'a> {
    data: &'a TimelineViewerData,
    selected_frame: usize,
    zoom_and_pan: &'a mut ZoomAndPanTransform,
    selected_trace_robot: &'a mut Option<PlayerNumber>,
    behavior_tree_visualizer: &'a mut BehaviorTreeVisualizer,
}

impl TabViewer for TimelineDockViewer<'_> {
    type Tab = TimelineViewerTab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            TimelineViewerTab::Field => {
                show_map(ui, self.data, self.selected_frame, self.zoom_and_pan);
            }
            TimelineViewerTab::BehaviorTree => {
                show_behavior_tree(
                    ui,
                    self.data,
                    self.selected_frame,
                    self.selected_trace_robot,
                    self.behavior_tree_visualizer,
                );
            }
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TimelineViewerTab::Field => "Field".into(),
            TimelineViewerTab::BehaviorTree => "Behavior Tree".into(),
        }
    }
}

fn show_behavior_tree(
    ui: &mut Ui,
    data: &TimelineViewerData,
    selected_frame: usize,
    selected_trace_robot: &mut Option<PlayerNumber>,
    behavior_tree_visualizer: &mut BehaviorTreeVisualizer,
) {
    let robot_numbers = data
        .frames
        .get(selected_frame)
        .map(|frame| frame.robot_frames.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    if robot_numbers.is_empty() {
        *selected_trace_robot = None;
        behavior_tree_visualizer.clear();
        ui.label("no robot traces in selected frame");
        return;
    }

    if selected_trace_robot.is_none_or(|robot| !robot_numbers.contains(&robot)) {
        *selected_trace_robot = robot_numbers.first().copied();
        behavior_tree_visualizer.clear();
    }

    let previous_robot = *selected_trace_robot;
    ComboBox::from_label("Robot")
        .selected_text(
            selected_trace_robot
                .map(|robot| robot.to_string())
                .unwrap_or_else(|| "none".to_string()),
        )
        .show_ui(ui, |ui| {
            for robot_number in robot_numbers {
                ui.selectable_value(
                    selected_trace_robot,
                    Some(robot_number),
                    robot_number.to_string(),
                );
            }
        });
    if *selected_trace_robot != previous_robot {
        behavior_tree_visualizer.clear();
    }

    let tree_data = selected_trace_robot.and_then(|robot_number| {
        data.frames
            .get(selected_frame)
            .and_then(|frame| frame.robot_frames.get(&robot_number))
    });

    let Some(robot_frame) = tree_data else {
        behavior_tree_visualizer.clear();
        ui.label("selected robot has no node trace");
        return;
    };

    behavior_tree_visualizer.show(
        ui,
        Some(&robot_frame.static_layout),
        Some(&robot_frame.trace),
    );
}

fn show_map(
    ui: &mut Ui,
    data: &TimelineViewerData,
    selected_frame: usize,
    zoom_and_pan: &mut ZoomAndPanTransform,
) {
    let available_size = ui.available_size_before_wrap();
    if available_size.x <= 1.0 || available_size.y <= 1.0 {
        ui.label("not enough space to draw field");
        return;
    }

    let field_dimensions = data.field_dimensions;
    let border = field_dimensions.border_strip_width;
    let (response, mut painter) = TwixPainter::<Field>::allocate(
        ui,
        vector![
            2.0 * border + field_dimensions.length,
            2.0 * border + field_dimensions.width
        ],
        point![
            border + field_dimensions.length / 2.0,
            -border - field_dimensions.width / 2.0
        ],
        Orientation::RightHanded,
    );

    zoom_and_pan.apply(ui, &mut painter, &response);
    painter.field(&field_dimensions);

    if let Some(frame) = data.frames.get(selected_frame) {
        if let Some(ball) = frame.ball {
            painter.ball(
                point_world_to_field(ball.position),
                field_dimensions.ball_radius,
                Color32::YELLOW,
            );
        }

        for (_, robot) in frame.robots.iter() {
            let Some(robot) = robot else {
                continue;
            };
            let pose = pose_world_to_field(robot.ground_to_world.as_pose());
            let color = robot_color(robot.player_number);
            painter.pose(
                pose,
                0.16,
                0.32,
                color,
                Stroke {
                    width: 0.025,
                    color: Color32::BLACK,
                },
            );
            paint_robot_label(ui, &painter, pose, robot.player_number);
        }
    }
}

fn frame_inspection_value(frame: &TimelineFrame) -> Value {
    let mut value = serde_json::to_value(frame)
        .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));
    summarize_large_json_values(&mut value);
    value
}

fn summarize_large_json_values(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if object.contains_key("tiles")
                && object.contains_key("width_tiles")
                && object.contains_key("height_tiles")
                && let Some(Value::Array(tiles)) = object.get("tiles")
            {
                object.insert(
                    "tiles".to_string(),
                    json!({
                        "summary": format!("{} voronoi tiles omitted", tiles.len()),
                        "count": tiles.len(),
                    }),
                );
            }

            for child in object.values_mut() {
                summarize_large_json_values(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                summarize_large_json_values(item);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn show_json_value(ui: &mut Ui, label: &str, value: &Value, path: &str, filter: &str) {
    if !json_value_matches(value, path, filter) {
        return;
    }

    match value {
        Value::Object(object) => {
            CollapsingHeader::new(format!("{label} {{ {} }}", object.len()))
                .default_open(!filter.is_empty())
                .show(ui, |ui| {
                    for (key, child) in object {
                        let child_path = format_json_path(path, key);
                        show_json_value(ui, key, child, &child_path, filter);
                    }
                });
        }
        Value::Array(items) => {
            CollapsingHeader::new(format!("{label} [{}]", items.len()))
                .default_open(!filter.is_empty())
                .show(ui, |ui| {
                    for (index, child) in items.iter().enumerate() {
                        let key = format!("[{index}]");
                        let child_path = format!("{path}{key}");
                        show_json_value(ui, &key, child, &child_path, filter);
                    }
                });
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            ui.add(
                Label::new(
                    RichText::new(format!("{label}: {}", format_scalar_value(value))).monospace(),
                )
                .wrap(),
            );
        }
    }
}

fn json_value_matches(value: &Value, path: &str, filter: &str) -> bool {
    if filter.is_empty() || path.to_ascii_lowercase().contains(filter) {
        return true;
    }

    match value {
        Value::Object(object) => object.iter().any(|(key, child)| {
            let child_path = format_json_path(path, key);
            json_value_matches(child, &child_path, filter)
        }),
        Value::Array(items) => items.iter().enumerate().any(|(index, child)| {
            let child_path = format!("{path}[{index}]");
            json_value_matches(child, &child_path, filter)
        }),
        Value::Null => "null".contains(filter),
        Value::Bool(value) => value.to_string().contains(filter),
        Value::Number(value) => value.to_string().contains(filter),
        Value::String(value) => value.to_ascii_lowercase().contains(filter),
    }
}

fn format_json_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn format_scalar_value(value: &Value) -> String {
    let text = match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    };
    const MAX_SCALAR_LENGTH: usize = 180;
    let mut chars = text.chars();
    let shortened = chars.by_ref().take(MAX_SCALAR_LENGTH).collect::<String>();
    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        text
    }
}

fn paint_robot_label(
    ui: &mut Ui,
    painter: &TwixPainter<Field>,
    pose: Pose2<Field>,
    player_number: PlayerNumber,
) {
    let label_position = painter.transform_world_to_pixel(pose.position() + vector![0.0, 0.26]);
    ui.painter().text(
        label_position,
        Align2::CENTER_CENTER,
        player_number.to_string(),
        FontId::proportional(14.0),
        Color32::WHITE,
    );
}

fn pose_world_to_field(pose: Pose2<World>) -> Pose2<Field> {
    Pose2::from_parts(
        point![pose.position().x(), pose.position().y()],
        Orientation2::new(pose.orientation().angle()),
    )
}

fn point_world_to_field(point: linear_algebra::Point2<World>) -> linear_algebra::Point2<Field> {
    point![point.x(), point.y()]
}

fn robot_color(player_number: PlayerNumber) -> Color32 {
    match player_number {
        PlayerNumber::One => Color32::from_rgb(80, 160, 255),
        PlayerNumber::Two => Color32::from_rgb(255, 128, 64),
        PlayerNumber::Three => Color32::from_rgb(128, 220, 128),
        PlayerNumber::Four => Color32::from_rgb(220, 128, 255),
        PlayerNumber::Five => Color32::from_rgb(255, 220, 80),
    }
}

fn format_time(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{:.3}s", duration.as_secs_f32()),
        Err(error) => format!("-{:.3}s", error.duration().as_secs_f32()),
    }
}

fn frame_duration_seconds(current: &TimelineFrame, next: &TimelineFrame) -> f64 {
    next.now
        .duration_since(current.now)
        .unwrap_or(Duration::from_millis(10))
        .max(Duration::from_millis(1))
        .as_secs_f64()
}

fn motion_name(motion_command: &MotionCommand) -> &'static str {
    match motion_command {
        MotionCommand::Damping => "damping",
        MotionCommand::Prepare => "prepare",
        MotionCommand::Stand { .. } => "stand",
        MotionCommand::StandUp => "stand_up",
        MotionCommand::VisualKick { .. } => "visual_kick",
        MotionCommand::Walk { .. } => "walk",
        MotionCommand::WalkWithVelocity { .. } => "walk_with_velocity",
    }
}
