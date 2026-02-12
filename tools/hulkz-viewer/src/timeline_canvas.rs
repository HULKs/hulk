use chrono::{DateTime, Local, Utc};
use eframe::egui::{self, Color32};
use std::time::Duration;

use crate::app::{LaneRenderRow, TimelineRenderRange};

const CANVAS_MIN_WIDTH: f32 = 420.0;
const CANVAS_HEIGHT: f32 = 236.0;
const INNER_MARGIN_PX: f32 = 10.0;
const AXIS_HEIGHT_PX: f32 = 30.0;
const LANE_RAIL_WIDTH_PX: f32 = 180.0;
const MIN_LANE_HEIGHT_PX: f32 = 12.0;

const OUTER_CORNER_RADIUS: f32 = 10.0;
const INNER_CORNER_RADIUS: f32 = 8.0;

const OUTER_BG: Color32 = Color32::from_rgb(19, 28, 34);
const INNER_BG: Color32 = Color32::from_rgb(27, 39, 47);
const ANCHOR_LINE_COLOR: Color32 = Color32::from_rgb(236, 132, 56);
const ANCHOR_DIAMOND_COLOR: Color32 = Color32::from_rgb(245, 167, 89);
const LABEL_COLOR: Color32 = Color32::from_rgb(177, 200, 208);
const MUTED_LABEL_COLOR: Color32 = Color32::from_rgb(136, 156, 165);

const LANE_COLORS: [Color32; 10] = [
    Color32::from_rgb(79, 177, 157),
    Color32::from_rgb(236, 132, 56),
    Color32::from_rgb(113, 173, 242),
    Color32::from_rgb(246, 194, 93),
    Color32::from_rgb(170, 132, 240),
    Color32::from_rgb(89, 208, 130),
    Color32::from_rgb(230, 116, 140),
    Color32::from_rgb(136, 190, 101),
    Color32::from_rgb(102, 199, 219),
    Color32::from_rgb(210, 153, 96),
];

#[derive(Debug, Clone, Copy)]
pub struct TimelineCanvasInput<'a> {
    pub full_range: TimelineRenderRange,
    pub viewport_range: TimelineRenderRange,
    pub anchor_timestamp_ns: u64,
    pub hover_timestamp_ns: Option<u64>,
    pub lane_rows: &'a [LaneRenderRow],
    pub lane_window_start: usize,
    pub total_lane_count: usize,
    pub lane_height_px: f32,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineCanvasOutput {
    pub hover_timestamp_ns: Option<u64>,
    pub selected_timestamp_ns: Option<u64>,
    pub pan_delta_fraction: Option<f32>,
    pub zoom_factor: Option<f32>,
    pub zoom_center_timestamp_ns: Option<u64>,
    pub lane_scroll_delta: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum WheelInteraction {
    LaneScroll(f32),
    Pan(f32),
    Zoom(f32),
}

pub fn timeline_lane_window_capacity(lane_height_px: f32) -> usize {
    ((lane_plot_height() / lane_height_px.max(MIN_LANE_HEIGHT_PX)).floor() as usize).max(1)
}

pub fn draw_timeline_canvas(
    ui: &mut egui::Ui,
    input: TimelineCanvasInput<'_>,
) -> TimelineCanvasOutput {
    let (timeline_rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(CANVAS_MIN_WIDTH), CANVAS_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(timeline_rect);

    painter.rect_filled(timeline_rect, OUTER_CORNER_RADIUS, OUTER_BG);

    let inner_rect = egui::Rect::from_min_max(
        timeline_rect.left_top() + egui::vec2(INNER_MARGIN_PX, INNER_MARGIN_PX),
        timeline_rect.right_bottom() - egui::vec2(INNER_MARGIN_PX, INNER_MARGIN_PX),
    );
    painter.rect_filled(inner_rect, INNER_CORNER_RADIUS, INNER_BG);

    let lane_track_rect = egui::Rect::from_min_max(
        inner_rect.left_top(),
        egui::pos2(inner_rect.right(), inner_rect.bottom() - AXIS_HEIGHT_PX),
    );
    let lane_rail_rect = egui::Rect::from_min_max(
        lane_track_rect.left_top(),
        egui::pos2(
            lane_track_rect.left() + LANE_RAIL_WIDTH_PX.min(lane_track_rect.width() * 0.4),
            lane_track_rect.bottom(),
        ),
    );
    let plot_rect = egui::Rect::from_min_max(
        egui::pos2(lane_rail_rect.right() + 4.0, lane_track_rect.top()),
        lane_track_rect.right_bottom(),
    );

    for step in 0..=6 {
        let t = step as f32 / 6.0;
        let x = egui::lerp(plot_rect.left()..=plot_rect.right(), t);
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, grid_color()),
        );
    }

    let lane_height = input.lane_height_px.max(MIN_LANE_HEIGHT_PX);
    let lane_clip_rect = lane_rail_rect.shrink2(egui::vec2(6.0, 2.0));
    let mut hovered_lane_label: Option<String> = None;
    let pointer_pos = response.hover_pos();
    for (row_index, row) in input.lane_rows.iter().enumerate() {
        let y_top = plot_rect.top() + row_index as f32 * lane_height;
        let y_bottom = (y_top + lane_height).min(plot_rect.bottom());
        if y_bottom < plot_rect.top() || y_top > plot_rect.bottom() {
            continue;
        }
        let y_center = (y_top + y_bottom) * 0.5;

        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), y_bottom),
                egui::pos2(plot_rect.right(), y_bottom),
            ],
            egui::Stroke::new(1.0, lane_grid_color()),
        );

        let label_color = if row.active_bindings > 0 {
            LABEL_COLOR
        } else {
            MUTED_LABEL_COLOR
        };
        painter.with_clip_rect(lane_clip_rect).text(
            egui::pos2(lane_clip_rect.left(), y_center),
            egui::Align2::LEFT_CENTER,
            row.label.as_str(),
            egui::FontId::monospace(10.5),
            label_color,
        );

        for point in &row.points {
            let x = x_for_timestamp(point.timestamp_ns, plot_rect, input.viewport_range);
            if x < plot_rect.left() - 2.0 || x > plot_rect.right() + 2.0 {
                continue;
            }
            let density_stretch = (1.0 + (point.count.max(1) as f32).log2()).clamp(1.0, 4.2);
            let fill = lane_color(row.color_index)
                .linear_multiply((0.42 + 0.11 * density_stretch).clamp(0.3, 0.95));
            draw_diamond(
                &painter,
                egui::pos2(x, y_center),
                3.8,
                3.3 * density_stretch,
                fill,
            );
        }

        if let Some(pointer) = pointer_pos {
            if lane_rail_rect.contains(pointer)
                && pointer.y >= y_top
                && pointer.y <= y_bottom
                && !row.label.is_empty()
            {
                hovered_lane_label = Some(format!(
                    "{} ({})",
                    row.key.path_expression, row.key.namespace
                ));
            }
        }
    }

    painter.line_segment(
        [
            egui::pos2(plot_rect.left(), plot_rect.bottom()),
            egui::pos2(plot_rect.right(), plot_rect.bottom()),
        ],
        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 48)),
    );
    painter.line_segment(
        [
            egui::pos2(lane_rail_rect.right(), plot_rect.top()),
            egui::pos2(lane_rail_rect.right(), plot_rect.bottom()),
        ],
        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
    );

    let viewport_span = range_span(input.viewport_range);
    let viewport_span_ns = duration_to_nanos(viewport_span);
    let timestamp_for_x = |x: f32| -> u64 {
        if plot_rect.width() <= f32::EPSILON {
            return input.viewport_range.start_ns;
        }
        let fraction = ((x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0);
        if viewport_span.is_zero() {
            input.viewport_range.start_ns
        } else {
            input
                .viewport_range
                .start_ns
                .saturating_add((fraction as f64 * viewport_span_ns as f64).round() as u64)
        }
    };

    let anchor_x = x_for_timestamp(input.anchor_timestamp_ns, plot_rect, input.viewport_range);
    painter.line_segment(
        [
            egui::pos2(anchor_x, plot_rect.top()),
            egui::pos2(anchor_x, plot_rect.bottom()),
        ],
        egui::Stroke::new(2.0, ANCHOR_LINE_COLOR),
    );
    draw_diamond(
        &painter,
        egui::pos2(anchor_x, plot_rect.top() + 5.0),
        4.2,
        4.2,
        ANCHOR_DIAMOND_COLOR,
    );

    let mut output = TimelineCanvasOutput {
        hover_timestamp_ns: input.hover_timestamp_ns,
        ..TimelineCanvasOutput::default()
    };

    if let Some(pointer_pos) = pointer_pos {
        if plot_rect.contains(pointer_pos) {
            let hover_ts = timestamp_for_x(pointer_pos.x);
            output.hover_timestamp_ns = Some(hover_ts);
            let hover_x = x_for_timestamp(hover_ts, plot_rect, input.viewport_range);
            painter.line_segment(
                [
                    egui::pos2(hover_x, plot_rect.top()),
                    egui::pos2(hover_x, plot_rect.bottom()),
                ],
                egui::Stroke::new(1.0, hover_line_color()),
            );
        }
    }

    let is_shift_pan_drag = response.dragged_by(egui::PointerButton::Primary)
        && ui.input(|input| input.modifiers.shift);
    let is_secondary_pan_drag = response.dragged_by(egui::PointerButton::Secondary);
    if is_shift_pan_drag || is_secondary_pan_drag {
        let pointer_delta_x = ui.input(|input| input.pointer.delta().x);
        let pan_fraction = pan_fraction_for_drag_delta(pointer_delta_x, plot_rect.width());
        if pan_fraction.abs() > f32::EPSILON {
            output.pan_delta_fraction = Some(pan_fraction);
        }
    } else if response.clicked_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Primary)
    {
        if let Some(pointer) = response.interact_pointer_pos() {
            if plot_rect.contains(pointer) {
                output.selected_timestamp_ns = Some(timestamp_for_x(pointer.x));
            }
        }
    }

    if response.hovered() {
        let (smooth_scroll_delta, raw_scroll_delta, modifiers) = ui.input(|input| {
            (
                input.smooth_scroll_delta,
                input.raw_scroll_delta,
                input.modifiers,
            )
        });
        let scroll_delta = if smooth_scroll_delta.length_sq() > f32::EPSILON {
            smooth_scroll_delta
        } else {
            raw_scroll_delta
        };
        if let Some(interaction) =
            wheel_interaction_from_scroll(scroll_delta, modifiers, plot_rect.width())
        {
            match interaction {
                WheelInteraction::LaneScroll(delta) => {
                    output.lane_scroll_delta = Some(delta);
                }
                WheelInteraction::Pan(fraction) => {
                    output.pan_delta_fraction = Some(fraction);
                }
                WheelInteraction::Zoom(zoom_factor) => {
                    output.zoom_factor = Some(zoom_factor);
                    output.zoom_center_timestamp_ns = output
                        .hover_timestamp_ns
                        .or(Some(input.anchor_timestamp_ns));
                }
            }
        }
    }

    if let Some(label) = hovered_lane_label {
        let _ = response.clone().on_hover_text(label);
    }

    let full_span = range_span(input.full_range);
    let full_span_ns = duration_to_nanos(full_span);
    if full_span_ns > 0 {
        let nav_track_y = plot_rect.bottom() + 6.0;
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), nav_track_y),
                egui::pos2(plot_rect.right(), nav_track_y),
            ],
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
        );
        let view_start_fraction = fraction_for_timestamp(
            input.viewport_range.start_ns,
            input.full_range.start_ns,
            full_span,
        );
        let view_end_fraction = fraction_for_timestamp(
            input.viewport_range.end_ns,
            input.full_range.start_ns,
            full_span,
        );
        let view_left = egui::lerp(plot_rect.left()..=plot_rect.right(), view_start_fraction);
        let view_right = egui::lerp(plot_rect.left()..=plot_rect.right(), view_end_fraction);
        painter.line_segment(
            [
                egui::pos2(view_left, nav_track_y),
                egui::pos2(view_right, nav_track_y),
            ],
            egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
        );
    }

    painter.text(
        egui::pos2(plot_rect.left(), inner_rect.bottom() - 18.0),
        egui::Align2::LEFT_TOP,
        format_timestamp_compact(input.viewport_range.start_ns),
        egui::FontId::monospace(10.0),
        LABEL_COLOR,
    );
    painter.text(
        egui::pos2(plot_rect.right(), inner_rect.bottom() - 18.0),
        egui::Align2::RIGHT_TOP,
        format_timestamp_compact(input.viewport_range.end_ns),
        egui::FontId::monospace(10.0),
        LABEL_COLOR,
    );

    let lane_end = input
        .lane_window_start
        .saturating_add(input.lane_rows.len())
        .min(input.total_lane_count);
    painter.text(
        egui::pos2(lane_rail_rect.left() + 2.0, inner_rect.bottom() - 18.0),
        egui::Align2::LEFT_TOP,
        format!(
            "lanes {}-{} / {}",
            input.lane_window_start.saturating_add(1),
            lane_end,
            input.total_lane_count
        ),
        egui::FontId::monospace(10.0),
        MUTED_LABEL_COLOR,
    );

    output
}

pub(crate) fn pan_fraction_for_drag_delta(pointer_delta_x: f32, width: f32) -> f32 {
    if width <= f32::EPSILON {
        return 0.0;
    }
    (-pointer_delta_x / width).clamp(-1.0, 1.0)
}

pub(crate) fn pan_fraction_for_scroll_delta(scroll_delta: f32, width: f32) -> f32 {
    if width <= f32::EPSILON {
        return 0.0;
    }
    (scroll_delta / width).clamp(-0.4, 0.4)
}

pub(crate) fn zoom_factor_for_scroll_delta(scroll_delta_y: f32) -> Option<f32> {
    if scroll_delta_y.abs() <= f32::EPSILON {
        return None;
    }
    let factor = (1.0 - scroll_delta_y * 0.0015).clamp(0.6, 1.6);
    Some(factor)
}

pub(crate) fn wheel_interaction_from_scroll(
    scroll_delta: egui::Vec2,
    modifiers: egui::Modifiers,
    plot_width: f32,
) -> Option<WheelInteraction> {
    let scroll_signal = if scroll_delta.y.abs() >= scroll_delta.x.abs() {
        scroll_delta.y
    } else {
        scroll_delta.x
    };
    if scroll_signal.abs() <= f32::EPSILON {
        return None;
    }

    if modifiers.ctrl {
        return zoom_factor_for_scroll_delta(scroll_signal).map(WheelInteraction::Zoom);
    }
    if modifiers.shift {
        let pan_fraction = pan_fraction_for_scroll_delta(scroll_signal, plot_width);
        if pan_fraction.abs() > f32::EPSILON {
            return Some(WheelInteraction::Pan(pan_fraction));
        }
        return None;
    }

    Some(WheelInteraction::LaneScroll(-scroll_signal))
}

fn lane_plot_height() -> f32 {
    CANVAS_HEIGHT - INNER_MARGIN_PX * 2.0 - AXIS_HEIGHT_PX
}

fn x_for_timestamp(
    timestamp_ns: u64,
    plot_rect: egui::Rect,
    viewport_range: TimelineRenderRange,
) -> f32 {
    let span = range_span(viewport_range);
    let fraction = fraction_for_timestamp(timestamp_ns, viewport_range.start_ns, span);
    egui::lerp(plot_rect.left()..=plot_rect.right(), fraction)
}

fn fraction_for_timestamp(timestamp: u64, start: u64, span: Duration) -> f32 {
    let span_nanos = duration_to_nanos(span);
    if span_nanos == 0 {
        1.0
    } else {
        (timestamp.saturating_sub(start) as f64 / span_nanos as f64).clamp(0.0, 1.0) as f32
    }
}

fn range_span(range: TimelineRenderRange) -> Duration {
    Duration::from_nanos(range.end_ns.saturating_sub(range.start_ns))
}

fn duration_to_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn lane_color(index: usize) -> Color32 {
    LANE_COLORS[index % LANE_COLORS.len()]
}

fn grid_color() -> Color32 {
    Color32::from_rgba_unmultiplied(158, 180, 189, 26)
}

fn lane_grid_color() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 24)
}

fn hover_line_color() -> Color32 {
    Color32::from_rgba_unmultiplied(235, 235, 235, 120)
}

fn draw_diamond(
    painter: &egui::Painter,
    center: egui::Pos2,
    x_radius: f32,
    y_radius: f32,
    color: Color32,
) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(center.x, center.y - y_radius),
            egui::pos2(center.x + x_radius, center.y),
            egui::pos2(center.x, center.y + y_radius),
            egui::pos2(center.x - x_radius, center.y),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

fn format_timestamp_compact(nanos: u64) -> String {
    let secs = nanos / 1_000_000_000;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    let Ok(secs_i64) = i64::try_from(secs) else {
        return format!("{nanos} ns");
    };
    let Some(utc) = DateTime::<Utc>::from_timestamp(secs_i64, subsec_nanos) else {
        return format!("{nanos} ns");
    };
    utc.with_timezone(&Local).format("%H:%M:%S%.3f").to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        pan_fraction_for_drag_delta, timeline_lane_window_capacity, wheel_interaction_from_scroll,
        zoom_factor_for_scroll_delta, WheelInteraction,
    };
    use eframe::egui::{Modifiers, Vec2};

    #[test]
    fn wheel_without_modifiers_scrolls_lanes() {
        let interaction =
            wheel_interaction_from_scroll(Vec2::new(0.0, -120.0), Modifiers::NONE, 500.0)
                .expect("lane scroll interaction");
        assert!(matches!(interaction, WheelInteraction::LaneScroll(delta) if delta > 0.0));
    }

    #[test]
    fn shift_wheel_maps_to_pan() {
        let interaction = wheel_interaction_from_scroll(
            Vec2::new(0.0, 120.0),
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            400.0,
        )
        .expect("pan interaction");
        assert!(matches!(interaction, WheelInteraction::Pan(_)));
    }

    #[test]
    fn ctrl_wheel_maps_to_zoom() {
        let interaction = wheel_interaction_from_scroll(
            Vec2::new(0.0, 120.0),
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            400.0,
        )
        .expect("zoom interaction");
        assert!(matches!(interaction, WheelInteraction::Zoom(_)));
    }

    #[test]
    fn zoom_direction_is_preserved() {
        let zoom_in = zoom_factor_for_scroll_delta(120.0).expect("zoom");
        let zoom_out = zoom_factor_for_scroll_delta(-120.0).expect("zoom");
        assert!(zoom_in < 1.0);
        assert!(zoom_out > 1.0);
    }

    #[test]
    fn drag_pan_direction_is_consistent() {
        assert!(pan_fraction_for_drag_delta(60.0, 200.0) < 0.0);
        assert!(pan_fraction_for_drag_delta(-60.0, 200.0) > 0.0);
    }

    #[test]
    fn lane_capacity_is_always_positive() {
        assert!(timeline_lane_window_capacity(16.0) >= 1);
        assert!(timeline_lane_window_capacity(42.0) >= 1);
    }
}
