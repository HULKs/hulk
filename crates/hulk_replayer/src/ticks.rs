use chrono::{DateTime, Utc};
use eframe::{
    egui::{
        pos2, vec2, Color32, FontId, Painter, Rect, Response, Sense, Shape, Stroke, TextStyle, Ui,
        Widget,
    },
    epaint::PathShape,
};

use crate::coordinate_systems::{
    AbsoluteScreen, AbsoluteTime, FrameRange, RelativeTime, ScreenRange, ViewportRange,
};

pub fn ticks_height(ui: &Ui) -> f32 {
    ui.style().text_styles.get(&TextStyle::Body).unwrap().size * 2.0
}

pub struct Ticks<'state> {
    frame_range: &'state FrameRange,
    viewport_range: &'state ViewportRange,
    position: &'state mut RelativeTime,
}

impl<'state> Ticks<'state> {
    pub fn new(
        frame_range: &'state FrameRange,
        viewport_range: &'state ViewportRange,
        position: &'state mut RelativeTime,
    ) -> Self {
        Self {
            frame_range,
            viewport_range,
            position,
        }
    }

    fn show_position_text(
        &self,
        painter: &Painter,
        font: FontId,
        position: AbsoluteScreen,
    ) -> Rect {
        let clip_rect = painter.clip_rect();
        let text =
            absolute_time_to_time_string(self.position.map_to_absolute_time(self.frame_range));
        let galley = painter.layout_no_wrap(text.clone(), font.clone(), Color32::GREEN);
        let text_width = galley.rect.width();
        let text_position = if text_width <= clip_rect.width() {
            if position.inner() - text_width / 2.0 < clip_rect.left() {
                pos2(clip_rect.left(), clip_rect.top())
            } else if position.inner() + text_width / 2.0 > clip_rect.right() {
                pos2(clip_rect.right() - text_width, clip_rect.top())
            } else {
                pos2(position.inner() - text_width / 2.0, clip_rect.top())
            }
        } else {
            pos2(position.inner() - text_width / 2.0, clip_rect.top())
        };
        let rect = Rect::from_min_size(text_position, vec2(text_width, galley.rect.height()));
        painter.galley(text_position, galley, Color32::GREEN);
        rect
    }

    fn show_arrow(&self, painter: &Painter, left: bool) {
        let clip_rect = &painter.clip_rect();
        let arrow_edge_translation = vec2(clip_rect.height() / 2.0, 0.0);
        painter.add(Shape::Path(PathShape::convex_polygon(
            if left {
                vec![
                    clip_rect.left_top() + arrow_edge_translation,
                    clip_rect.left_bottom() + arrow_edge_translation,
                    clip_rect.left_center(),
                ]
            } else {
                vec![
                    clip_rect.right_top() - arrow_edge_translation,
                    clip_rect.right_center(),
                    clip_rect.right_bottom() - arrow_edge_translation,
                ]
            },
            Color32::GREEN,
            Stroke::NONE,
        )));
    }

    fn show_ticks(
        &self,
        painter: &Painter,
        font: FontId,
        color_strong: Color32,
        color_weak: Color32,
        position_text_rect: Option<Rect>,
        screen_range: &ScreenRange,
    ) {
        let clip_rect = painter.clip_rect();
        let spacing_lower_bound = AbsoluteScreen::new(clip_rect.height())
            .scale_to_relative_screen(screen_range)
            .scale_to_relative_time(self.viewport_range);
        let spacing_log10 = spacing_lower_bound.inner().log10();
        let spacing_small = RelativeTime::new(10.0_f32.powf(spacing_log10.ceil()));
        let spacing_large = RelativeTime::new(10.0_f32.powf(spacing_log10.ceil() + 1.0));
        let left_outside = self.viewport_range.start()
            - self.viewport_range.start() % spacing_large.inner()
            - spacing_large;

        let mut current = left_outside;
        let mut number_of_ticks = 0;
        while current < self.viewport_range.end() + spacing_large {
            let x = current
                .map_to_relative_screen(self.viewport_range)
                .map_to_absolute_screen(screen_range);
            let is_strong = number_of_ticks % 5 == 0;
            painter.line_segment(
                [
                    pos2(x.inner(), clip_rect.center().y + clip_rect.height() * 0.125),
                    pos2(x.inner(), clip_rect.bottom()),
                ],
                Stroke::new(1.0, if is_strong { color_strong } else { color_weak }),
            );
            if is_strong {
                let text =
                    absolute_time_to_time_string(current.map_to_absolute_time(self.frame_range));
                let galley = painter.layout_no_wrap(text.clone(), font.clone(), color_strong);
                let text_position = pos2(x.inner() - galley.rect.width() / 2.0, clip_rect.top());
                if position_text_rect.map_or(true, |position_text_rect| {
                    !galley
                        .rect
                        .translate(text_position.to_vec2())
                        .intersects(position_text_rect)
                }) {
                    painter.galley(text_position, galley, color_strong);
                }
            }
            current += spacing_small;
            number_of_ticks += 1;
        }
    }

    fn show_position_tick(&self, painter: &Painter, position: AbsoluteScreen) {
        let clip_rect = painter.clip_rect();
        painter.line_segment(
            [
                pos2(
                    position.inner(),
                    clip_rect.center().y + clip_rect.height() * 0.125,
                ),
                pos2(position.inner(), clip_rect.bottom()),
            ],
            Stroke::new(2.0, Color32::GREEN),
        );
    }
}

impl<'state> Widget for Ticks<'state> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut size = ui.available_size();
        size.y = ticks_height(ui);
        let (response, painter) = ui.allocate_painter(size, Sense::hover());

        let font = ui
            .style()
            .text_styles
            .get(&TextStyle::Body)
            .unwrap()
            .clone();

        let screen_range = ScreenRange::new(
            AbsoluteScreen::new(painter.clip_rect().left()),
            AbsoluteScreen::new(painter.clip_rect().right()),
        );

        let position_is_left = *self.position < self.viewport_range.start();
        let position_is_right = *self.position > self.viewport_range.end();
        let position_visible = !position_is_left && !position_is_right;
        let position = self
            .position
            .map_to_relative_screen(self.viewport_range)
            .map_to_absolute_screen(&screen_range);

        let position_text_rect = if position_visible {
            Some(self.show_position_text(&painter, font.clone(), position))
        } else {
            None
        };

        if position_is_left || position_is_right {
            self.show_arrow(&painter, position_is_left);
        }

        self.show_ticks(
            &painter,
            font,
            ui.visuals().strong_text_color(),
            ui.visuals().weak_text_color(),
            position_text_rect,
            &screen_range,
        );

        if position_visible {
            self.show_position_tick(&painter, position);
        }

        response
    }
}

fn absolute_time_to_time_string(absolute_time: AbsoluteTime) -> String {
    Into::<DateTime<Utc>>::into(absolute_time.inner())
        .format("%H:%M:%S%.3f")
        .to_string()
}
