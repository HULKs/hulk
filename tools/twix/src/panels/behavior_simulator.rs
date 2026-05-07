use std::sync::Arc;

use eframe::egui::{Align, Color32, Layout, Response, Slider, Ui, Widget};
use hulk_widgets::SegmentedControl;
use serde_json::{Value, json};

use crate::{
    backend::BackendCapability,
    log_error::LogError,
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    value_buffer::BufferHandle,
};

pub struct BehaviorSimulatorPanel {
    robot: Arc<Robot>,
    unavailable_reason: Option<&'static str>,

    selected_frame: f64,
    selected_robot: usize,
    playing: bool,
    playback_speed: f64,

    selected_frame_updater: Option<BufferHandle<usize>>,
    frame_count: Option<BufferHandle<usize>>,
}

impl<'a> Panel<'a> for BehaviorSimulatorPanel {
    const NAME: &'static str = "Behavior Simulator";

    fn new(context: PanelCreationContext) -> Self {
        let supports_panel = context.robot.has_capability(BackendCapability::ValueWrite)
            && context
                .robot
                .has_capability(BackendCapability::TypedSubscription);
        let (selected_frame_updater, frame_count, unavailable_reason) = if supports_panel {
            (
                Some(context.robot.subscribe_value("simulator.selected_frame")),
                Some(
                    context
                        .robot
                        .subscribe_value("BehaviorSimulator.main_outputs.frame_count"),
                ),
                None,
            )
        } else {
            (
                None,
                None,
                Some(
                    "Behavior Simulator is unavailable on this backend because simulator writes are not supported.",
                ),
            )
        };
        let selected_frame = context
            .value
            .and_then(|value| value.get("selected_frame"))
            .and_then(|value| value.as_f64())
            .unwrap_or_default();
        let selected_robot = context
            .value
            .and_then(|value| value.get("selected_robot"))
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize;
        let playing = context
            .value
            .and_then(|value| value.get("playing"))
            .and_then(|value| value.as_bool())
            .unwrap_or_default();

        Self {
            robot: context.robot,
            unavailable_reason,

            selected_frame,
            selected_robot,
            playing,
            playback_speed: 5.0,

            selected_frame_updater,
            frame_count,
        }
    }

    fn save(&self) -> Value {
        json!({
            "selected_frame": self.selected_frame.clone(),
            "selected_robot": self.selected_robot.clone(),
            "playing": self.playing.clone(),
        })
    }
}

impl Widget for &mut BehaviorSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Some(reason) = self.unavailable_reason {
            return ui.label(reason);
        }

        if self.selected_frame_updater.as_ref().unwrap().has_changed() {
            self.selected_frame_updater.as_mut().unwrap().mark_as_seen();
            if !self.playing
                && let Some(selected_frame) = self
                    .selected_frame_updater
                    .as_ref()
                    .unwrap()
                    .get_last_value()
                    .ok()
                    .flatten()
            {
                self.selected_frame = selected_frame as f64
            }
        }
        let frame_count = match self.frame_count.as_ref().unwrap().get_last_value() {
            Ok(Some(frame_count)) => frame_count,
            Ok(None) => return ui.label("no frame data"),
            Err(error) => return ui.colored_label(Color32::RED, format!("Error: {error}")),
        };
        let mut new_frame = None;
        let response = ui
            .vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                    let mut frame = self.selected_frame as usize;
                    if ui
                        .add_sized(
                            ui.available_size(),
                            Slider::new(&mut frame, 0..=frame_count - 1)
                                .smart_aim(false)
                                .text("Frame"),
                        )
                        .changed()
                    {
                        new_frame = Some(frame as f64);
                    };
                });
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.add_space(50.0);

                        ui.add(
                            Slider::new(&mut self.playback_speed, -10.0..=10.0)
                                .step_by(0.1)
                                .text("Playback Speed"),
                        );

                        ui.add_space(50.0);

                        let robots = (1..=7).collect::<Vec<_>>();
                        let response = SegmentedControl::new(
                            "robot-selector",
                            &mut self.selected_robot,
                            &robots,
                        )
                        .ui(ui);
                        if response.changed() {
                            self.robot
                                .write(
                                    "simulator.selected_robot",
                                    robots[self.selected_robot].into(),
                                )
                                .log_err();
                        };
                    });
                });
                ui.checkbox(&mut self.playing, "Play")
            })
            .response;

        if self.playing {
            let elapsed = ui.input(|input| input.stable_dt as f64);
            let frames_per_second = 1000.0 / 12.0 * self.playback_speed;
            new_frame = Some(self.selected_frame + frames_per_second * elapsed);
        }
        if ui.button(">>").clicked() {
            new_frame = Some(new_frame.unwrap_or(self.selected_frame) + 10.0);
        }
        if let Some(new_frame) = new_frame {
            self.selected_frame = (new_frame + frame_count as f64) % frame_count as f64;
            self.robot
                .write(
                    "simulator.selected_frame",
                    (self.selected_frame as usize).into(),
                )
                .log_err();
        }
        response
    }
}
