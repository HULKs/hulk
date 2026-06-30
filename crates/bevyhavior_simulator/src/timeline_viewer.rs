use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::{
    Result,
    eyre::{Report, WrapErr},
};
use coordinate_systems::{Field, Ground, World};
use eframe::{
    App, Frame, NativeOptions,
    egui::{
        Align2, CentralPanel, CollapsingHeader, ComboBox, Context, Event, FontId, Key, Label, Pos2,
        Rect, RichText, ScrollArea, Sense, SidePanel, Slider, StrokeKind, TextEdit, Tooltip,
        TopBottomPanel, Ui, Vec2, WidgetText, pos2,
    },
    epaint::{Color32, Stroke},
    run_native,
};
use egui_dock::{DockArea, DockState, Node, Split, TabViewer};
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Orientation2, Pose2, point, vector};
use serde_json::{Value, json};
use twix::{
    behavior_tree::BehaviorTreeVisualizer,
    twix_painter::{Orientation, TwixPainter},
    zoom_and_pan::ZoomAndPanTransform,
};
use types::{
    field_dimensions::FieldDimensions, filtered_game_state::FilteredGameState,
    motion_command::MotionCommand, obstacles::ObstacleKind, path::traits::EndPoints,
};

use crate::behavior_tree_simulator::{
    SimulationConfig, SimulatorFailure, SimulatorObstacle, SimulatorRobotId,
    SimulatorTimelineMarker, TimelineFrame,
};

const SCRUBBER_MARGIN: f32 = 8.0;
const SCRUBBER_HEIGHT_FACTOR: f32 = 2.0;
const MARKER_OVERHANG: f32 = 5.0;
const MIN_PLAYBACK_SPEED: f32 = 1.0 / 8.0;
const MAX_PLAYBACK_SPEED: f32 = 256.0;
const WALK_PATH_LINE_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 0, 202, 150);
const WALK_PATH_ARC_COLOR: Color32 = Color32::from_rgba_premultiplied(136, 170, 182, 150);

#[derive(Debug)]
pub struct TimelineViewerData {
    pub field_dimensions: FieldDimensions,
    pub config: SimulationConfig,
    pub frames: Vec<TimelineFrame>,
    pub markers: Vec<SimulatorTimelineMarker>,
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
    loop_playback: bool,
    playback_speed: f32,
    last_playback_update: Instant,
    playback_time_accumulator: f64,
    inspector_filter: String,
    inspector_cache_frame: Option<usize>,
    inspector_cache: Option<Value>,
    zoom_and_pan: ZoomAndPanTransform,
    selected_trace_robot: Option<SimulatorRobotId>,
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
            2.0 / 3.0,
            Node::leaf(TimelineViewerTab::BehaviorTree),
        );

        Self {
            data,
            selected_frame: 0,
            is_playing: false,
            loop_playback: true,
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

        if self.data.frames.len() <= 1 {
            self.is_playing = false;
            self.playback_time_accumulator = 0.0;
            return;
        }

        if self.selected_frame + 1 >= self.data.frames.len() {
            if self.loop_playback {
                self.selected_frame = 0;
                self.playback_time_accumulator = 0.0;
                self.last_playback_update = Instant::now();
                context.request_repaint();
            } else {
                self.is_playing = false;
                self.playback_time_accumulator = 0.0;
            }
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

        if self.selected_frame + 1 >= self.data.frames.len() && !self.loop_playback {
            self.is_playing = false;
            self.playback_time_accumulator = 0.0;
        } else {
            context.request_repaint();
        }
    }

    fn handle_hotkeys(&mut self, context: &Context) {
        if context.wants_keyboard_input() {
            return;
        }

        let (
            toggle_playback,
            faster,
            slower,
            previous_frame,
            next_frame,
            previous_second,
            next_second,
        ) = context.input(|input| {
            let text_input = |text: &str| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::Text(input_text) if input_text == text))
            };
            (
                input.key_pressed(Key::Space),
                text_input(">"),
                text_input("<"),
                text_input(","),
                text_input("."),
                input.key_pressed(Key::ArrowLeft),
                input.key_pressed(Key::ArrowRight),
            )
        });

        if toggle_playback {
            self.toggle_playback();
        }
        if faster {
            self.set_playback_speed(self.playback_speed * 2.0);
        }
        if slower {
            self.set_playback_speed(self.playback_speed / 2.0);
        }
        if previous_frame {
            self.select_previous_frame();
        }
        if next_frame {
            self.select_next_frame();
        }
        if previous_second {
            self.select_previous_second();
        }
        if next_second {
            self.select_next_second();
        }
    }

    fn toggle_playback(&mut self) {
        self.is_playing = !self.is_playing;
        self.reset_playback_time();
    }

    fn set_playback_speed(&mut self, playback_speed: f32) {
        self.playback_speed = playback_speed.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED);
    }

    fn select_previous_frame(&mut self) {
        self.set_selected_frame(self.selected_frame.saturating_sub(1));
    }

    fn select_next_frame(&mut self) {
        self.set_selected_frame(self.selected_frame.saturating_add(1));
    }

    fn select_previous_second(&mut self) {
        let Some(current_frame) = self.data.frames.get(self.selected_frame) else {
            return;
        };
        let target_time = current_frame
            .now
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(|| self.data.frames.first().expect("frames is not empty").now);
        let frame_index = self
            .data
            .frames
            .iter()
            .rposition(|frame| frame.now <= target_time)
            .unwrap_or(0);
        self.set_selected_frame(frame_index);
    }

    fn select_next_second(&mut self) {
        let Some(current_frame) = self.data.frames.get(self.selected_frame) else {
            return;
        };
        let target_time = current_frame
            .now
            .checked_add(Duration::from_secs(1))
            .unwrap_or_else(|| self.data.frames.last().expect("frames is not empty").now);
        let frame_index = self
            .data
            .frames
            .iter()
            .position(|frame| frame.now >= target_time)
            .unwrap_or_else(|| self.data.frames.len().saturating_sub(1));
        self.set_selected_frame(frame_index);
    }

    fn set_selected_frame(&mut self, selected_frame: usize) {
        let Some(last_frame_index) = self.data.frames.len().checked_sub(1) else {
            self.selected_frame = 0;
            self.reset_playback_time();
            return;
        };
        self.selected_frame = selected_frame.min(last_frame_index);
        self.reset_playback_time();
    }

    fn reset_playback_time(&mut self) {
        self.last_playback_update = Instant::now();
        self.playback_time_accumulator = 0.0;
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
                    for (robot_id, robot_frame) in &frame.robot_frames {
                        ui.label(format!(
                            "robot {robot_id}: {}",
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
                    self.toggle_playback();
                }

                ui.add(
                    Slider::new(
                        &mut self.playback_speed,
                        MIN_PLAYBACK_SPEED..=MAX_PLAYBACK_SPEED,
                    )
                    .logarithmic(true)
                    .text("speed")
                    .suffix("x"),
                );

                ui.checkbox(&mut self.loop_playback, "loop");

                if ui.button("previous").clicked() {
                    self.select_previous_frame();
                }

                if ui.button("next").clicked() {
                    self.select_next_frame();
                }

                if self.data.frames.is_empty() {
                    ui.label("no frames recorded");
                } else {
                    let scrubber_response = timeline_scrubber(
                        ui,
                        &self.data.frames,
                        &self.data.markers,
                        &mut self.selected_frame,
                    );
                    if scrubber_response.changed {
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

struct TimelineScrubberResponse {
    changed: bool,
}

fn timeline_scrubber(
    ui: &mut Ui,
    frames: &[TimelineFrame],
    markers: &[SimulatorTimelineMarker],
    selected_frame: &mut usize,
) -> TimelineScrubberResponse {
    let desired_size = Vec2::new(
        ui.available_width(),
        ui.spacing().interact_size.y * SCRUBBER_HEIGHT_FACTOR + SCRUBBER_MARGIN * 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
    let scrubber_rect = rect.shrink2(Vec2::splat(SCRUBBER_MARGIN));

    paint_timeline_scrubber(ui, scrubber_rect, frames, markers, *selected_frame);

    let mut changed = false;
    if (response.clicked() || response.dragged())
        && let Some(pointer_position) = response.interact_pointer_pos()
    {
        let next_frame = frame_index_at_x(frames, scrubber_rect, pointer_position.x);
        if *selected_frame != next_frame {
            *selected_frame = next_frame;
            changed = true;
        }
    }

    if response.hovered()
        && let Some(pointer_position) = ui.ctx().pointer_hover_pos()
    {
        let hover_frame = frame_index_at_x(frames, scrubber_rect, pointer_position.x);
        let hovered_markers = markers_at_x(frames, markers, scrubber_rect, pointer_position);
        let tooltip_position = pos2(pointer_position.x, scrubber_rect.top() - 8.0);
        let tooltip = Tooltip::always_open(
            ui.ctx().clone(),
            response.layer_id,
            response.id.with("timeline_scrubber_hover"),
            tooltip_position,
        );
        tooltip.show(|ui| {
            for marker in &hovered_markers {
                ui.colored_label(marker.color, marker.label.as_str());
            }

            if let Some(frame) = frames.get(hover_frame) {
                if !hovered_markers.is_empty() {
                    ui.separator();
                }
                ui.label(format!("frame: {hover_frame}"));
                ui.label(format!("time: {}", format_time(frame.now)));
                ui.label(format!("state: {}", game_state_name(frame.game_state)));
            }
        });
    }

    TimelineScrubberResponse { changed }
}

fn paint_timeline_scrubber(
    ui: &Ui,
    rect: Rect,
    frames: &[TimelineFrame],
    markers: &[SimulatorTimelineMarker],
    selected_frame: usize,
) {
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, Color32::DARK_GRAY);

    if frames.is_empty() {
        return;
    }

    if frames.len() == 1 {
        painter.rect_filled(rect, 3.0, game_state_color(frames[0].game_state));
    } else {
        for (index, frame) in frames.iter().enumerate() {
            let left = frame_x(frames, rect, index);
            let right = if index + 1 < frames.len() {
                frame_x(frames, rect, index + 1)
            } else {
                rect.right()
            };
            if right > left {
                painter.rect_filled(
                    Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom())),
                    0.0,
                    game_state_color(frame.game_state),
                );
            }
        }
    }

    for marker in markers {
        let x = time_x(frames, rect, marker.frame_time);
        painter.line_segment(
            [
                pos2(x, rect.top() - MARKER_OVERHANG),
                pos2(x, rect.bottom() + MARKER_OVERHANG),
            ],
            Stroke::new(2.0_f32, marker.color),
        );
    }

    let selected_x = frame_x(frames, rect, selected_frame.min(frames.len() - 1));
    painter.line_segment(
        [
            pos2(selected_x, rect.top() - 2.0),
            pos2(selected_x, rect.bottom() + 2.0),
        ],
        Stroke::new(2.0_f32, Color32::WHITE),
    );
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0_f32, Color32::BLACK),
        StrokeKind::Inside,
    );
}

fn markers_at_x<'a>(
    frames: &[TimelineFrame],
    markers: &'a [SimulatorTimelineMarker],
    rect: Rect,
    pointer_position: Pos2,
) -> Vec<&'a SimulatorTimelineMarker> {
    if !rect.expand(6.0).contains(pointer_position) {
        return Vec::new();
    }

    markers
        .iter()
        .filter(|marker| {
            (time_x(frames, rect, marker.frame_time) - pointer_position.x).abs() <= 6.0
        })
        .collect()
}

fn frame_index_at_x(frames: &[TimelineFrame], rect: Rect, x: f32) -> usize {
    frames
        .iter()
        .enumerate()
        .min_by(|(left_index, _), (right_index, _)| {
            let left_distance = (frame_x(frames, rect, *left_index) - x).abs();
            let right_distance = (frame_x(frames, rect, *right_index) - x).abs();
            left_distance.total_cmp(&right_distance)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn frame_x(frames: &[TimelineFrame], rect: Rect, frame_index: usize) -> f32 {
    if frames.len() <= 1 {
        return rect.left();
    }

    let Some(frame) = frames.get(frame_index) else {
        return rect.right();
    };
    time_x(frames, rect, frame.now)
}

fn time_x(frames: &[TimelineFrame], rect: Rect, frame_time: SystemTime) -> f32 {
    if frames.len() <= 1 {
        return rect.left();
    }

    let start = frames.first().expect("frames is not empty").now;
    let end = frames.last().expect("frames is not empty").now;
    let total_duration = end.duration_since(start).unwrap_or_default().as_secs_f32();
    if total_duration <= f32::EPSILON {
        return rect.left();
    }

    let elapsed = frame_time
        .duration_since(start)
        .unwrap_or_default()
        .as_secs_f32();
    rect.left() + rect.width() * (elapsed / total_duration).clamp(0.0, 1.0)
}

fn game_state_color(game_state: FilteredGameState) -> Color32 {
    match game_state {
        FilteredGameState::Initial => Color32::from_gray(90),
        FilteredGameState::Ready => Color32::from_rgb(80, 150, 255),
        FilteredGameState::Set => Color32::from_rgb(255, 190, 60),
        FilteredGameState::Playing { .. } => Color32::from_rgb(80, 190, 95),
        FilteredGameState::Finished => Color32::from_rgb(160, 100, 220),
        FilteredGameState::Stop => Color32::from_rgb(220, 70, 70),
    }
}

fn game_state_name(game_state: FilteredGameState) -> &'static str {
    match game_state {
        FilteredGameState::Initial => "Initial",
        FilteredGameState::Ready => "Ready",
        FilteredGameState::Set => "Set",
        FilteredGameState::Playing { .. } => "Playing",
        FilteredGameState::Finished => "Finished",
        FilteredGameState::Stop => "Stop",
    }
}

impl App for TimelineViewerApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        self.clamp_selected_frame();
        self.handle_hotkeys(context);
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
    selected_trace_robot: &'a mut Option<SimulatorRobotId>,
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
    selected_trace_robot: &mut Option<SimulatorRobotId>,
    behavior_tree_visualizer: &mut BehaviorTreeVisualizer,
) {
    let robot_ids = data
        .frames
        .get(selected_frame)
        .map(|frame| frame.robot_frames.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    if robot_ids.is_empty() {
        *selected_trace_robot = None;
        behavior_tree_visualizer.clear();
        ui.label("no robot traces in selected frame");
        return;
    }

    if selected_trace_robot.is_none_or(|robot| !robot_ids.contains(&robot)) {
        *selected_trace_robot = robot_ids.first().copied();
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
            for robot_id in robot_ids {
                ui.selectable_value(selected_trace_robot, Some(robot_id), robot_id.to_string());
            }
        });
    if *selected_trace_robot != previous_robot {
        behavior_tree_visualizer.clear();
    }

    let tree_data = selected_trace_robot.and_then(|robot_id| {
        data.frames
            .get(selected_frame)
            .and_then(|frame| frame.robot_frames.get(&robot_id))
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

        for obstacle in &frame.scenario_obstacles {
            paint_scenario_obstacle(&painter, *obstacle);
        }

        for (robot_id, robot) in frame.robots.iter() {
            let pose = pose_world_to_field(robot.ground_to_world.as_pose());
            let color = robot_color(*robot_id);
            if let Some(robot_frame) = frame.robot_frames.get(robot_id) {
                paint_walk_path(&painter, pose, &robot_frame.motion_command);
            }
            paint_view_cone(&painter, pose, robot.head_yaw, &data.config, color);
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
            paint_robot_label(ui, &painter, pose, *robot_id);
        }
    }
}

fn paint_scenario_obstacle(painter: &TwixPainter<Field>, obstacle: SimulatorObstacle) {
    let color = scenario_obstacle_color(obstacle.kind);
    painter.circle(
        point_world_to_field(obstacle.position),
        obstacle.radius_at_foot_height,
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60),
        Stroke { width: 0.02, color },
    );
}

fn scenario_obstacle_color(kind: ObstacleKind) -> Color32 {
    match kind {
        ObstacleKind::Ball => Color32::YELLOW,
        ObstacleKind::GoalPost => Color32::WHITE,
        ObstacleKind::Robot => Color32::LIGHT_RED,
        ObstacleKind::Person => Color32::LIGHT_GREEN,
        ObstacleKind::Unknown => Color32::LIGHT_GRAY,
    }
}

fn paint_walk_path(
    painter: &TwixPainter<Field>,
    pose: Pose2<Field>,
    motion_command: &MotionCommand,
) {
    let MotionCommand::Walk {
        path,
        target_orientation,
        ..
    } = motion_command
    else {
        return;
    };

    let ground_to_field = pose.as_transform::<Ground>();
    let ground_painter = painter.transform_painter(ground_to_field.inverse());
    ground_painter.path(
        path.clone(),
        WALK_PATH_LINE_COLOR,
        WALK_PATH_ARC_COLOR,
        0.025,
    );

    let path_end_point = path.end_point();
    let target_direction = target_orientation.as_unit_vector();
    ground_painter.line_segment(
        path_end_point,
        path_end_point + target_direction * 0.1,
        Stroke::new(0.01_f32, Color32::PURPLE),
    );
}

fn paint_view_cone(
    painter: &TwixPainter<Field>,
    pose: Pose2<Field>,
    head_yaw: Orientation2<Ground>,
    config: &SimulationConfig,
    color: Color32,
) {
    let center = pose.position();
    let direction = pose.orientation().angle() + head_yaw.angle();
    let half_angle = config.visibility_field_of_view / 2.0;
    let range = config.ball_visibility_range;
    let stroke = Stroke {
        width: 0.015,
        color: Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 160),
    };
    let left = center + Orientation2::new(direction + half_angle).as_unit_vector() * range;
    let right = center + Orientation2::new(direction - half_angle).as_unit_vector() * range;

    painter.line_segment(center, left, stroke);
    painter.line_segment(center, right, stroke);

    let segments = 24;
    let arc_points = (0..=segments).map(|index| {
        let factor = index as f32 / segments as f32;
        let angle = direction - half_angle + factor * config.visibility_field_of_view;
        center + Orientation2::new(angle).as_unit_vector() * range
    });
    painter.polyline(arc_points, stroke);
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
    robot_id: SimulatorRobotId,
) {
    let label_position = painter.transform_world_to_pixel(pose.position() + vector![0.0, 0.26]);
    ui.painter().text(
        label_position,
        Align2::CENTER_CENTER,
        robot_id.to_string(),
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

fn robot_color(robot_id: SimulatorRobotId) -> Color32 {
    let base = match robot_id.player_number {
        PlayerNumber::One => Color32::from_rgb(80, 160, 255),
        PlayerNumber::Two => Color32::from_rgb(255, 128, 64),
        PlayerNumber::Three => Color32::from_rgb(128, 220, 128),
        PlayerNumber::Four => Color32::from_rgb(220, 128, 255),
        PlayerNumber::Five => Color32::from_rgb(255, 220, 80),
    };
    match robot_id.team {
        Team::Hulks => base,
        Team::Opponent => Color32::from_rgb(base.r() / 2, base.g() / 2, base.b() / 2),
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
