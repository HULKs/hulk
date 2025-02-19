use std::collections::BTreeMap;

use eframe::egui::{
    pos2, vec2, Color32, CornerRadius, Key, Painter, PointerButton, Pos2, Rect, Response, Sense,
    Stroke, Ui, Vec2, Widget,
};

use framework::Timing;

use crate::coordinate_systems::{
    AbsoluteScreen, AbsoluteTime, FrameRange, RelativeTime, ScreenRange, ViewportRange,
};

pub struct Frames<'state> {
    indices: &'state BTreeMap<String, Vec<Timing>>,
    frame_range: &'state FrameRange,
    viewport_range: &'state mut ViewportRange,
    position: &'state mut RelativeTime,
    item_spacing: Vec2,
}

impl<'state> Frames<'state> {
    pub fn new(
        indices: &'state BTreeMap<String, Vec<Timing>>,
        frame_range: &'state FrameRange,
        viewport_range: &'state mut ViewportRange,
        position: &'state mut RelativeTime,
        item_spacing: Vec2,
    ) -> Self {
        Self {
            indices,
            frame_range,
            viewport_range,
            position,
            item_spacing,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn interact(
        &mut self,
        double_clicked: bool,
        cursor_position: Option<Pos2>,
        cursor_down: bool,
        scroll_delta: Vec2,
        shift_down: bool,
        keys: Keys,
        screen_range: &ScreenRange,
    ) -> bool {
        if double_clicked {
            *self.viewport_range = ViewportRange::from_frame_range(self.frame_range);
            return false;
        }

        let cursor_position =
            AbsoluteScreen::new(cursor_position.map_or(0.0, |position| position.x))
                .map_to_relative_screen(screen_range);

        let position_changed_by_click = {
            let cursor_position = cursor_position.map_to_relative_time(self.viewport_range);
            let position_changed = cursor_down && cursor_position != *self.position;
            if position_changed {
                *self.position = cursor_position;
            }
            position_changed
        };

        let (zoom_offset, viewport_length) = if shift_down {
            (
                RelativeTime::new(0.0),
                self.viewport_range.end() - self.viewport_range.start(),
            )
        } else {
            let zoom_factor = 0.99_f32.powf(scroll_delta.y);
            let previous_viewport_length = self.viewport_range.end() - self.viewport_range.start();
            let viewport_length = previous_viewport_length * zoom_factor;
            let viewport_length_difference = viewport_length - previous_viewport_length;
            let zoom_offset = -viewport_length_difference * cursor_position.inner();
            (zoom_offset, viewport_length)
        };

        let pan_offset =
            AbsoluteScreen::new(scroll_delta.x + if shift_down { scroll_delta.y } else { 0.0 })
                .scale_to_relative_screen(screen_range)
                .scale_to_relative_time(self.viewport_range);

        let viewport_start = self.viewport_range.start() + pan_offset + zoom_offset;
        *self.viewport_range = ViewportRange::new(viewport_start, viewport_start + viewport_length);

        if keys.jump_backward_large {
            *self.position -= RelativeTime::new(10.0);
        }
        if keys.jump_forward_large {
            *self.position += RelativeTime::new(10.0);
        }
        if keys.jump_backward_small {
            *self.position -= RelativeTime::new(1.0);
        }
        if keys.jump_forward_small {
            *self.position += RelativeTime::new(1.0);
        }
        if keys.step_backward {
            *self.position -= RelativeTime::new(0.01);
        }
        if keys.step_forward {
            *self.position += RelativeTime::new(0.01);
        }

        position_changed_by_click
            || keys.jump_backward_large
            || keys.jump_forward_large
            || keys.jump_backward_small
            || keys.jump_forward_small
            || keys.step_backward
            || keys.step_forward
    }

    fn show_cyclers(&self, painter: &Painter, color: Color32, screen_range: &ScreenRange) {
        let spacing = self.item_spacing.y;
        let total_spacing = spacing * (self.indices.len() - 1) as f32;
        let row_height = (painter.clip_rect().height() - total_spacing) / self.indices.len() as f32;

        for (index, recording_index) in self.indices.values().enumerate() {
            let top_left =
                painter.clip_rect().left_top() + vec2(0.0, (row_height + spacing) * index as f32);
            let mut painter = painter.clone();
            painter.set_clip_rect(Rect::from_min_max(
                top_left,
                pos2(painter.clip_rect().right(), top_left.y + row_height),
            ));
            self.show_cycler(recording_index, painter, color, screen_range);
        }
    }

    fn show_cycler(
        &self,
        index: &[Timing],
        painter: Painter,
        color: Color32,
        screen_range: &ScreenRange,
    ) {
        for frame in index {
            self.show_frame(frame, &painter, color, screen_range);
        }
    }

    fn show_frame(
        &self,
        frame: &Timing,
        painter: &Painter,
        color: Color32,
        screen_range: &ScreenRange,
    ) {
        let left = AbsoluteTime::new(frame.timestamp)
            .map_to_relative_time(self.frame_range)
            .map_to_relative_screen(self.viewport_range)
            .map_to_absolute_screen(screen_range);
        let right = AbsoluteTime::new(frame.timestamp + frame.duration)
            .map_to_relative_time(self.frame_range)
            .map_to_relative_screen(self.viewport_range)
            .map_to_absolute_screen(screen_range);

        let mut rect = painter.clip_rect();
        rect.set_left(left.inner());
        rect.set_right(right.inner());

        painter.rect_filled(rect, CornerRadius::ZERO, color);
    }

    fn show_position(&self, painter: &Painter, color: Color32, screen_range: &ScreenRange) {
        let clip_rect = painter.clip_rect();
        let x = self
            .position
            .map_to_relative_screen(self.viewport_range)
            .map_to_absolute_screen(screen_range);

        painter.line_segment(
            [
                pos2(x.inner(), clip_rect.top()),
                pos2(x.inner(), clip_rect.bottom()),
            ],
            Stroke::new(2.0, color),
        );
    }
}

impl Widget for Frames<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        let screen_range = ScreenRange::new(
            AbsoluteScreen::new(painter.clip_rect().left()),
            AbsoluteScreen::new(painter.clip_rect().right()),
        );

        let (double_clicked, cursor_position, cursor_down, scroll_delta, shift_down, keys) = ui
            .input(|input| {
                (
                    input.pointer.button_double_clicked(PointerButton::Primary),
                    input.pointer.interact_pos(),
                    input.pointer.button_down(PointerButton::Primary),
                    input.smooth_scroll_delta,
                    input.modifiers.shift,
                    Keys {
                        jump_backward_large: input.key_pressed(Key::J)
                            || input.key_pressed(Key::ArrowDown),
                        jump_forward_large: input.key_pressed(Key::L)
                            || input.key_pressed(Key::ArrowUp),
                        jump_backward_small: input.key_pressed(Key::ArrowLeft),
                        jump_forward_small: input.key_pressed(Key::ArrowRight),
                        step_backward: input.key_pressed(Key::Comma),
                        step_forward: input.key_pressed(Key::Period),
                    },
                )
            });

        if self.interact(
            double_clicked,
            cursor_position,
            cursor_down,
            scroll_delta,
            shift_down,
            keys,
            &screen_range,
        ) {
            response.mark_changed();
        }

        self.show_cyclers(&painter, ui.visuals().strong_text_color(), &screen_range);
        self.show_position(&painter, Color32::GREEN, &screen_range);

        response
    }
}

struct Keys {
    jump_backward_large: bool,
    jump_forward_large: bool,
    jump_backward_small: bool,
    jump_forward_small: bool,
    step_backward: bool,
    step_forward: bool,
}
