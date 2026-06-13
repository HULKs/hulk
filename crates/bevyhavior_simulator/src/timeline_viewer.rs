use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use color_eyre::{
    Result,
    eyre::{Report, WrapErr},
};
use coordinate_systems::Field;
use eframe::{
    App, Frame, NativeOptions,
    egui::{Align2, CentralPanel, Context, FontId, SidePanel, Slider, TopBottomPanel, Ui},
    epaint::{Color32, Stroke},
    run_native,
};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Pose2, point, vector};
use twix::{
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
    zoom_and_pan: ZoomAndPanTransform,
}

impl TimelineViewerApp {
    fn new(data: TimelineViewerData) -> Self {
        Self {
            data,
            selected_frame: 0,
            is_playing: false,
            playback_speed: 1.0,
            last_playback_update: Instant::now(),
            playback_time_accumulator: 0.0,
            zoom_and_pan: ZoomAndPanTransform::default(),
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

    fn show_side_panel(&self, context: &Context) {
        SidePanel::right("timeline_viewer_side_panel")
            .resizable(true)
            .default_width(260.0)
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
                            ui.colored_label(
                                Color32::LIGHT_RED,
                                format!(
                                    "{} {:?}: {}",
                                    violation.check_name,
                                    violation.player_number,
                                    violation.message
                                ),
                            );
                        }
                    }
                }

                if !self.data.failures.is_empty() {
                    ui.separator();
                    ui.heading("Scenario Failures");
                    for failure in &self.data.failures {
                        ui.label(format_failure(failure));
                    }
                }
            });
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

    fn show_map(&mut self, context: &Context) {
        CentralPanel::default().show(context, |ui| {
            let field_dimensions = self.data.field_dimensions;
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

            self.zoom_and_pan.apply(ui, &mut painter, &response);
            painter.field(&field_dimensions);

            if let Some(frame) = self.selected_frame() {
                if let Some(ball) = frame.ball {
                    painter.ball(ball.position, field_dimensions.ball_radius, Color32::YELLOW);
                }

                for (_, robot) in frame.robots.iter() {
                    let Some(robot) = robot else {
                        continue;
                    };
                    let pose = robot.ground_to_field.as_pose();
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
        self.show_map(context);
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
        MotionCommand::Prepare => "prepare",
        MotionCommand::Stand { .. } => "stand",
        MotionCommand::StandUp => "stand_up",
        MotionCommand::VisualKick { .. } => "visual_kick",
        MotionCommand::Walk { .. } => "walk",
        MotionCommand::WalkWithVelocity { .. } => "walk_with_velocity",
    }
}

fn format_failure(failure: &SimulatorFailure) -> String {
    match failure {
        SimulatorFailure::InvariantViolation(violation) => format!(
            "{} {:?}: {}",
            violation.check_name, violation.player_number, violation.message
        ),
        SimulatorFailure::ScenarioAssertion(message) => message.clone(),
    }
}
