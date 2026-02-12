use eframe::egui;

use crate::{
    app::{format_timestamp, is_manual_timeline_navigation},
    timeline_canvas::{draw_timeline_canvas, timeline_lane_window_capacity, TimelineCanvasInput},
};

use super::{Panel, ViewerApp};

pub(super) struct TimelinePanel;

impl Panel for TimelinePanel {
    type State = ();

    fn draw(app: &mut ViewerApp, ui: &mut egui::Ui, _state: &mut Self::State) {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut app.follow_live, "Follow live").changed() && app.follow_live {
                app.jump_latest_internal(true);
            }

            if ui.button("Prev").clicked() {
                app.mark_manual_timeline_navigation();
                if let Some(index) = app.global_timeline_index {
                    app.set_global_timeline_index(index.saturating_sub(1), true);
                } else if !app.global_timeline.is_empty() {
                    app.set_global_timeline_index(
                        app.global_timeline.len().saturating_sub(1),
                        true,
                    );
                }
            }

            if ui.button("Next").clicked() {
                app.mark_manual_timeline_navigation();
                if !app.global_timeline.is_empty() {
                    let max_index = app.global_timeline.len().saturating_sub(1);
                    let index = app.global_timeline_index.unwrap_or(0).min(max_index);
                    app.set_global_timeline_index(index.saturating_add(1).min(max_index), true);
                }
            }

            if ui.button("Jump Latest").clicked() {
                app.follow_live = true;
                app.jump_latest_internal(true);
            }
        });
        ui.small("Wheel: lanes. Shift+Wheel: pan. Ctrl+Wheel: zoom.");
        ui.separator();

        if app.global_timeline.is_empty() {
            app.set_timeline_hover_preview(None);
            ui.label("No timeline points yet.");
            return;
        }

        if app.global_timeline_index.is_none() {
            app.jump_latest_internal(false);
        }

        let Some(full_range) = app.timeline_full_range() else {
            return;
        };
        let Some(viewport_range) = app.timeline_render_range() else {
            return;
        };
        let Some(anchor_ts) = app.current_anchor_nanos() else {
            return;
        };

        let lane_window_count = timeline_lane_window_capacity(app.timeline_viewport.lane_height_px);
        let lane_window_start = app.timeline_viewport.lane_scroll_offset.floor() as usize;
        let (lane_rows, total_lanes) = app.timeline_lane_rows(
            viewport_range,
            lane_window_start,
            lane_window_count,
            ui.available_width(),
        );

        let canvas_output = draw_timeline_canvas(
            ui,
            TimelineCanvasInput {
                full_range,
                viewport_range,
                anchor_timestamp_ns: anchor_ts,
                hover_timestamp_ns: app.timeline_hover_preview,
                lane_rows: lane_rows.as_slice(),
                lane_window_start,
                total_lane_count: total_lanes,
                lane_height_px: app.timeline_viewport.lane_height_px,
            },
        );

        if is_manual_timeline_navigation(
            canvas_output.selected_timestamp_ns,
            canvas_output.pan_delta_fraction,
            canvas_output.zoom_factor,
            canvas_output.lane_scroll_delta,
        ) {
            app.mark_manual_timeline_navigation();
        }
        if let Some(selected_timestamp_ns) = canvas_output.selected_timestamp_ns {
            app.set_global_timeline_anchor_by_timestamp(selected_timestamp_ns, true);
        }
        if let Some(pan_delta_fraction) = canvas_output.pan_delta_fraction {
            app.apply_timeline_pan_fraction(pan_delta_fraction);
        }
        if let Some(zoom_factor) = canvas_output.zoom_factor {
            let focus_timestamp_ns = canvas_output.zoom_center_timestamp_ns.unwrap_or(anchor_ts);
            app.apply_timeline_zoom(zoom_factor, focus_timestamp_ns);
        }
        if let Some(lane_scroll_delta) = canvas_output.lane_scroll_delta {
            app.apply_timeline_lane_scroll(lane_scroll_delta, total_lanes);
        }
        app.set_timeline_hover_preview(canvas_output.hover_timestamp_ns);

        if let Some(hover_ts) = app.timeline_hover_preview {
            ui.small(format!("Hover {}", format_timestamp(hover_ts)));
        } else {
            ui.small(format!("Anchor {}", format_timestamp(anchor_ts)));
        }
    }
}
