use crate::{
    app::{
        format_timestamp, is_manual_timeline_navigation,
        panel_prelude::{egui, Panel, PanelContext, UiIntent},
    },
    timeline_canvas::{draw_timeline_canvas, timeline_lane_window_capacity, TimelineCanvasInput},
};

pub struct TimelinePane;

impl Panel for TimelinePane {
    type State = ();

    fn draw(ctx: &mut PanelContext<'_>, ui: &mut egui::Ui, _state: &mut Self::State) {
        let mut follow_live = ctx.app().ui.follow_live;
        ui.horizontal(|ui| {
            if ui.checkbox(&mut follow_live, "Follow live").changed() {
                ctx.emit(UiIntent::TimelineSetFollowLive(follow_live));
            }

            if ui.button("Prev").clicked() {
                let prev_target = {
                    let app = ctx.app();
                    if let Some(index) = app.timeline.global_timeline_index {
                        let target_index = index.saturating_sub(1);
                        app.timeline.global_timeline.get(target_index).copied()
                    } else {
                        app.timeline.global_timeline.last().copied()
                    }
                };
                if let Some(target_timestamp) = prev_target {
                    ctx.emit(UiIntent::TimelineSelectAnchor(target_timestamp));
                }
            }

            if ui.button("Next").clicked() {
                let next_target = {
                    let app = ctx.app();
                    if app.timeline.global_timeline.is_empty() {
                        None
                    } else {
                        let max_index = app.timeline.global_timeline.len().saturating_sub(1);
                        let index = app
                            .timeline
                            .global_timeline_index
                            .unwrap_or(0)
                            .min(max_index);
                        let target_index = index.saturating_add(1).min(max_index);
                        app.timeline.global_timeline.get(target_index).copied()
                    }
                };
                if let Some(target_timestamp) = next_target {
                    ctx.emit(UiIntent::TimelineSelectAnchor(target_timestamp));
                }
            }

            if ui.button("Jump Latest").clicked() {
                ctx.emit(UiIntent::TimelineJumpLatest);
            }
        });
        ui.small("Wheel: lanes. Shift+Wheel: pan. Ctrl+Wheel: zoom.");
        ui.separator();

        let (canvas_output, anchor_ts, total_lanes, canvas_height) = {
            let app = ctx.app_mut();
            if app.timeline.global_timeline.is_empty() {
                app.set_timeline_hover_preview(None);
                ui.label("No timeline points yet.");
                return;
            }

            if app.timeline.global_timeline_index.is_none() {
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

            let hover_or_anchor_text = if let Some(hover_ts) = app.timeline.timeline_hover_preview {
                format!("Hover {}", format_timestamp(hover_ts))
            } else {
                format!("Anchor {}", format_timestamp(anchor_ts))
            };
            ui.small(hover_or_anchor_text);

            let canvas_height = ui.available_height();
            let lane_window_count = timeline_lane_window_capacity(
                app.timeline.timeline_viewport.lane_height_px,
                canvas_height,
            );
            let lane_window_start =
                app.timeline.timeline_viewport.lane_scroll_offset.floor() as usize;
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
                    hover_timestamp_ns: app.timeline.timeline_hover_preview,
                    lane_rows: lane_rows.as_slice(),
                    lane_window_start,
                    total_lane_count: total_lanes,
                    lane_height_px: app.timeline.timeline_viewport.lane_height_px,
                    canvas_height_px: canvas_height,
                },
            );

            app.set_timeline_hover_preview(canvas_output.hover_timestamp_ns);
            if let Some(lane_scroll_delta) = canvas_output.lane_scroll_delta {
                app.apply_timeline_lane_scroll(lane_scroll_delta, total_lanes, canvas_height);
            }

            (canvas_output, anchor_ts, total_lanes, canvas_height)
        };

        if is_manual_timeline_navigation(
            canvas_output.selected_timestamp_ns,
            canvas_output.pan_delta_fraction,
            canvas_output.zoom_factor,
        ) {
            ctx.emit(UiIntent::TimelineSetFollowLive(false));
        }
        if let Some(selected_timestamp_ns) = canvas_output.selected_timestamp_ns {
            ctx.emit(UiIntent::TimelineSelectAnchor(selected_timestamp_ns));
        }
        if let Some(pan_delta_fraction) = canvas_output.pan_delta_fraction {
            ctx.emit(UiIntent::TimelinePan(pan_delta_fraction));
        }
        if let Some(zoom_factor) = canvas_output.zoom_factor {
            let focus_timestamp_ns = canvas_output.zoom_center_timestamp_ns.unwrap_or(anchor_ts);
            ctx.emit(UiIntent::TimelineZoom {
                factor: zoom_factor,
                focus_ns: focus_timestamp_ns,
            });
        }
        if let Some(lane_scroll_delta) = canvas_output.lane_scroll_delta {
            ctx.emit(UiIntent::TimelineLaneScroll(lane_scroll_delta));
        }
        let _ = (total_lanes, canvas_height);
    }
}
