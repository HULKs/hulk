use std::{collections::BTreeMap, ops::RangeInclusive, time::Duration};

use chrono::{DateTime, Utc};
use eframe::{
    egui::{
        pos2, Align2, CentralPanel, Color32, Context, FontId, Painter, Pos2, Rect, Rounding, Sense,
        Slider, Stroke, TextStyle, TopBottomPanel, Vec2,
    },
    Frame,
};
use framework::{RecordingIndex, Timing};

use crate::{execution::Replayer, ReplayerHardwareInterface};

pub struct ReplayerApplication {
    replayer: Replayer<ReplayerHardwareInterface>,
    timing: Timing,
    viewport: Viewport,
    position: f32,
}

impl ReplayerApplication {
    pub fn new(replayer: Replayer<ReplayerHardwareInterface>) -> Self {
        let timing = join_timing(&replayer);
        Self {
            replayer,
            timing,
            viewport: Viewport {
                offset: 0.0,
                length: timing.duration.as_secs_f32(),
            },
            position: 0.0,
        }
    }

    fn update_viewport(
        &mut self,
        rect: Rect,
        drag_delta: Vec2,
        cursor_position: Option<Pos2>,
        scroll_delta: Vec2,
    ) {
        let previous_viewport_length = self.viewport.length;

        self.viewport.length *= 0.99_f32.powf(scroll_delta.y);

        let viewport_length_difference = self.viewport.length - previous_viewport_length;
        let cursor_position_relative =
            (cursor_position.map_or(0.0, |position| position.x) - rect.left()) / rect.width();
        let zoom_offset = -viewport_length_difference * cursor_position_relative;

        let scroll_offset = self.viewport.length * (scroll_delta.x / rect.width());
        let drag_offset = self.viewport.length * (-drag_delta.x / rect.width());
        self.viewport.offset += scroll_offset + drag_offset + zoom_offset;
    }

    fn render_labels(&self, painter: Painter, color: Color32, font: &FontId) -> f32 {
        let clip_rect = painter.clip_rect();
        let recording_indices = self.replayer.get_recording_indices();
        let cycler_instance_names = recording_indices.keys().collect::<Vec<_>>();
        let row_height = painter.clip_rect().height() / cycler_instance_names.len() as f32;
        let mut maximum_x = clip_rect.left();
        for (index, name) in cycler_instance_names.into_iter().enumerate() {
            let text_rect = painter.text(
                pos2(
                    clip_rect.min.x,
                    clip_rect.min.y + index as f32 * row_height + (row_height / 2.0),
                ),
                Align2::LEFT_CENTER,
                name,
                font.clone(),
                color,
            );
            maximum_x = maximum_x.max(text_rect.right());
        }
        maximum_x
    }

    fn render_heading(&self, painter: Painter, color: Color32, font: &FontId) {
        let viewport_left = self.viewport.offset;
        let viewport_center = self.viewport.offset + self.viewport.length * 0.5;
        let viewport_right = self.viewport.offset + self.viewport.length;
        let viewport_left_duration = Duration::from_secs_f32(viewport_left.abs());
        let viewport_center_duration = Duration::from_secs_f32(viewport_center.abs());
        let viewport_right_duration = Duration::from_secs_f32(viewport_right.abs());
        let viewport_left = if viewport_left > 0.0 {
            self.timing.timestamp + viewport_left_duration
        } else {
            self.timing.timestamp - viewport_left_duration
        };
        let viewport_center = if viewport_center > 0.0 {
            self.timing.timestamp + viewport_center_duration
        } else {
            self.timing.timestamp - viewport_center_duration
        };
        let viewport_right = if viewport_right > 0.0 {
            self.timing.timestamp + viewport_right_duration
        } else {
            self.timing.timestamp - viewport_right_duration
        };
        painter.text(
            painter.clip_rect().left_center(),
            Align2::LEFT_CENTER,
            format!(
                "{}",
                Into::<DateTime<Utc>>::into(viewport_left).format("%Y-%m-%d %H:%M:%S%.3f")
            ),
            font.clone(),
            color,
        );
        painter.text(
            painter.clip_rect().center(),
            Align2::CENTER_CENTER,
            format!(
                "{} ({:.3}s)",
                Into::<DateTime<Utc>>::into(viewport_center).format("%Y-%m-%d %H:%M:%S%.3f"),
                self.viewport.length
            ),
            font.clone(),
            color,
        );
        painter.text(
            painter.clip_rect().right_center(),
            Align2::RIGHT_CENTER,
            format!(
                "{}",
                Into::<DateTime<Utc>>::into(viewport_right).format("%Y-%m-%d %H:%M:%S%.3f")
            ),
            font.clone(),
            color,
        );
    }

    fn render_cyclers_frames(&self, painter: Painter, color: Color32) {
        let recording_indices = self.replayer.get_recording_indices();
        let row_height = painter.clip_rect().height() / recording_indices.len() as f32;
        for (index, recording_index) in recording_indices.values().enumerate() {
            let mut clip_rect = painter.clip_rect();
            clip_rect.set_top(clip_rect.min.y + index as f32 * row_height);
            clip_rect.set_height(row_height);
            self.render_cycler_frames(
                recording_index,
                {
                    let mut painter = painter.clone();
                    painter.set_clip_rect(clip_rect);
                    painter
                },
                color,
            );
        }
    }

    fn render_cycler_frames(&self, index: &RecordingIndex, painter: Painter, color: Color32) {
        for frame in index.iter() {
            self.render_cycler_frame(&frame, painter.clone(), color);
        }
    }

    fn render_cycler_frame(&self, frame: &Timing, painter: Painter, color: Color32) {
        let frame_left = frame
            .timestamp
            .duration_since(self.timing.timestamp)
            .expect("time ran backwards")
            .as_secs_f32();
        let frame_right = (frame.timestamp + frame.duration)
            .duration_since(self.timing.timestamp)
            .expect("time ran backwards")
            .as_secs_f32();
        let relative_left = (frame_left - self.viewport.offset) / self.viewport.length;
        let relative_right = (frame_right - self.viewport.offset) / self.viewport.length;
        let mut rect = painter.clip_rect();
        let left = rect.left() + relative_left * rect.width();
        let right = rect.left() + relative_right * rect.width();
        rect.set_left(left);
        rect.set_right(right);
        painter.rect_filled(rect, Rounding::ZERO, color);
    }

    fn render_position(&self, painter: Painter, color: Color32) {
        let x_relative = (self.position - self.viewport.offset) / self.viewport.length;
        let clip_rect = painter.clip_rect();
        let x = clip_rect.left() + x_relative * clip_rect.width();
        painter.line_segment(
            [pos2(x, clip_rect.top()), pos2(x, clip_rect.bottom())],
            Stroke::new(2.0, color),
        );
    }

    fn replay_at_position(&mut self) {
        let position_duration = Duration::from_secs_f32(self.position.abs());
        let timestamp = if self.position > 0.0 {
            self.timing.timestamp + position_duration
        } else {
            self.timing.timestamp - position_duration
        };
        let recording_indices = self.replayer.get_recording_indices_mut();
        let frames = recording_indices
            .into_iter()
            .map(|(name, index)| {
                (
                    name,
                    index
                        .find_latest_frame_up_to(timestamp)
                        .expect("failed to find latest frame"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (name, frame) in frames {
            if let Some(frame) = frame {
                self.replayer
                    .replay(&name, frame.timing.timestamp, &frame.data)
                    .expect("failed to replay frame");
            }
        }
    }
}

fn join_timing(replayer: &Replayer<ReplayerHardwareInterface>) -> Timing {
    let recording_indices = replayer.get_recording_indices();
    let begin = recording_indices
        .values()
        .flat_map(|index| index.first_timing().map(|timing| timing.timestamp))
        .min()
        .expect("there isn't any index that contains at least one frame");
    let end = recording_indices
        .values()
        .flat_map(|index| {
            index
                .last_timing()
                .map(|timing| timing.timestamp + timing.duration)
        })
        .max()
        .expect("there isn't any index that contains at least one frame");
    Timing {
        timestamp: begin,
        duration: end.duration_since(begin).expect("time ran backwards"),
    }
}

impl eframe::App for ReplayerApplication {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("Bärbel").show(context, |ui| {
            ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
            let changed = ui
                .add(
                    Slider::new(
                        &mut self.position,
                        RangeInclusive::new(0.0, self.timing.duration.as_secs_f32()),
                    )
                    .step_by(0.01),
                )
                .changed();
            if changed {
                self.replay_at_position();
            }
        });
        CentralPanel::default().show(context, |ui| {
            let font = ui
                .style()
                .text_styles
                .get(&TextStyle::Button)
                .unwrap()
                .clone();
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

            if response.double_clicked() {
                self.viewport = Viewport {
                    offset: 0.0,
                    length: self.timing.duration.as_secs_f32(),
                };
            }
            let drag_delta = response.drag_delta();
            let (cursor_position, scroll_delta) =
                ui.input(|input| (input.pointer.interact_pos(), input.scroll_delta));

            let color = Color32::WHITE;
            let clip_rect = {
                let mut clip_rect = painter.clip_rect();
                clip_rect.set_top(clip_rect.top() + 20.0);
                clip_rect
            };
            let divider_x = self.render_labels(
                {
                    let mut painter = painter.clone();
                    painter.set_clip_rect(clip_rect);
                    painter
                },
                color,
                &font,
            );
            let clip_rect = {
                let mut clip_rect = painter.clip_rect();
                clip_rect.set_left(divider_x + 5.0);
                clip_rect.set_top(clip_rect.top() + 20.0);
                clip_rect
            };
            if let Some(cursor_position) = cursor_position {
                if response.clicked() && clip_rect.contains(cursor_position) {
                    let cursor_position_relative =
                        (cursor_position.x - clip_rect.left()) / clip_rect.width();
                    self.position =
                        cursor_position_relative * self.viewport.length + self.viewport.offset;
                    self.replay_at_position();
                }
            }
            self.update_viewport(clip_rect, drag_delta, cursor_position, scroll_delta);
            let cyclers_painter = {
                let mut painter = painter.clone();
                painter.set_clip_rect(clip_rect);
                painter
            };
            self.render_cyclers_frames(cyclers_painter.clone(), color);
            self.render_position(cyclers_painter, Color32::GREEN);
            let clip_rect = {
                let mut clip_rect = painter.clip_rect();
                clip_rect.set_left(divider_x + 5.0);
                clip_rect.set_height(20.0);
                clip_rect
            };
            self.render_heading(
                {
                    let mut painter = painter.clone();
                    painter.set_clip_rect(clip_rect);
                    painter
                },
                color,
                &font,
            );
        });
    }
}

struct Viewport {
    offset: f32,
    length: f32,
}
