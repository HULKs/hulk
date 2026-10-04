use eframe::egui::{
    self, Align2, Button, Color32, DragValue, FontId, Key, Modifiers, PointerButton, Rect,
    Response, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};
use egui_material_icons::icons;
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    replay::ReplaySession,
};

pub struct TimelinePanel {
    step_seconds: f64,
    view: Option<TimeView>,
    recording: Option<String>,
}

/// Seconds relative to the recording start, keeping Unix timestamps out of f32 math.
#[derive(Clone, Copy, Debug)]
struct TimeView {
    start: f64,
    span: f64,
}

impl TimeView {
    fn fit(duration: f64) -> Self {
        let extent = duration.max(1.0);
        Self {
            start: -extent * 0.05,
            span: extent * 1.1,
        }
    }

    fn time_at(self, rect: Rect, x: f32) -> f64 {
        self.start + f64::from((x - rect.left()) / rect.width().max(1.0)) * self.span
    }

    fn x_at(self, rect: Rect, time: f64) -> f32 {
        rect.left() + ((time - self.start) / self.span) as f32 * rect.width()
    }

    fn constrain(&mut self, duration: f64) {
        let extent = duration.max(1.0);
        self.span = self.span.clamp(0.001, extent * 1.2);
        let first = -extent * 0.1;
        self.start = self
            .start
            .clamp(first, (extent * 1.1 - self.span).max(first));
    }

    fn zoom(&mut self, factor: f64, anchor: f64, duration: f64) {
        let fraction = (anchor - self.start) / self.span;
        self.span *= factor;
        self.constrain(duration);
        self.start = anchor - fraction * self.span;
        self.constrain(duration);
    }
}

impl Panel for TimelinePanel {
    const STORAGE_ID: &'static str = "timeline";
    const DISPLAY_NAME: &'static str = "Timeline";
    const ICON: &'static str = icons::ICON_TIMELINE.codepoint;

    fn new(context: PanelCreationContext<'_>) -> Self {
        Self {
            step_seconds: context
                .value
                .and_then(|value| value.get("step_seconds"))
                .and_then(Value::as_f64)
                .unwrap_or(1.0)
                .clamp(0.001, 60.0),
            view: None,
            recording: None,
        }
    }

    fn header_ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.toolbar(ui, &mut context.backend.replay());
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.timeline(
            ui,
            &mut context.backend.replay(),
            &context.backend.namespace(),
        );
    }

    fn save(&self) -> Value {
        json!({ "step_seconds": self.step_seconds })
    }
}

impl TimelinePanel {
    fn toolbar(&mut self, ui: &mut Ui, replay: &mut ReplaySession) {
        let Some(status) = replay.status.as_ref() else {
            ui.weak("Timeline");
            if let Some(error) = &replay.error {
                if ui.small_button("Retry").on_hover_text(error).clicked() {
                    replay.retry();
                }
            } else {
                ui.spinner();
            }
            return;
        };
        let (start, end, playing) = (status.start, status.end, status.playing);
        let position = replay.position().unwrap_or(status.position);
        let width = ui.available_width();
        let connected = replay.error.is_none();
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.add_enabled_ui(connected, |ui| {
            if transport_button(
                ui,
                icons::ICON_SKIP_PREVIOUS.codepoint,
                "Start (Home)",
                false,
            ) {
                replay.seek(start);
            }
            let step = (self.step_seconds * 1e9).round() as u64;
            if transport_button(
                ui,
                icons::ICON_FAST_REWIND.codepoint,
                "Step backward (Left)",
                false,
            ) {
                replay.seek(position.saturating_sub(step));
            }
            ui.add_enabled_ui(!replay.is_seeking() && (playing || position < end), |ui| {
                let (icon, label) = if playing && !replay.is_seeking() {
                    (icons::ICON_PAUSE.codepoint, "Pause (Space)")
                } else {
                    (icons::ICON_PLAY_ARROW.codepoint, "Play (Space)")
                };
                if transport_button(ui, icon, label, true) {
                    replay.set_playing(!playing);
                }
            });
            if transport_button(
                ui,
                icons::ICON_FAST_FORWARD.codepoint,
                "Step forward (Right)",
                false,
            ) {
                replay.seek(position.saturating_add(step));
            }
            if transport_button(ui, icons::ICON_SKIP_NEXT.codepoint, "End (End)", false) {
                replay.seek(end);
            }
        });
        if width >= 320.0 {
            ui.separator();
            ui.label(RichText::new(timestamp(position.saturating_sub(start))).monospace())
                .on_hover_text("Current time from recording start");
        }
        if width >= 470.0 {
            ui.label(
                RichText::new(format!("/ {}", timestamp(end - start)))
                    .monospace()
                    .weak(),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("⋯", |ui| {
                ui.add_enabled_ui(connected, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Jump to");
                        let mut seconds = position.saturating_sub(start) as f64 / 1e9;
                        if ui
                            .add(
                                DragValue::new(&mut seconds)
                                    .range(0.0..=(end - start) as f64 / 1e9)
                                    .speed(0.1)
                                    .max_decimals(3)
                                    .suffix(" s"),
                            )
                            .changed()
                        {
                            replay.seek(start + ((seconds * 1e9).round() as u64).min(end - start));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Step");
                    ui.add(
                        DragValue::new(&mut self.step_seconds)
                            .range(0.001..=60.0)
                            .speed(0.05)
                            .max_decimals(3)
                            .suffix(" s"),
                    );
                });
                if ui.button("Frame all (F)").clicked() {
                    self.view = None;
                    ui.close();
                }
                if ui.button("Copy timestamp").clicked() {
                    ui.ctx().copy_text(position.to_string());
                    ui.close();
                }
                ui.separator();
                ui.weak("Drag: scrub · wheel: zoom · middle drag: pan");
                ui.weak("Shift + wheel: pan · Shift + arrows: fine step");
            })
            .response
            .on_hover_text("Timeline options");
            if transport_button(ui, icons::ICON_FIT_SCREEN.codepoint, "Frame all (F)", false) {
                self.view = None;
            }
            if !connected {
                if ui
                    .small_button("Retry")
                    .on_hover_text(replay.error.as_deref().unwrap_or_default())
                    .clicked()
                {
                    replay.retry();
                }
            } else if width >= 580.0 {
                ui.weak(if playing && !replay.is_seeking() {
                    "Playing"
                } else {
                    "Paused"
                });
            }
        });
    }

    fn timeline(&mut self, ui: &mut Ui, replay: &mut ReplaySession, namespace: &str) {
        let size = ui.available_size().max(vec2(1.0, 1.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let Some(status) = replay.status.as_ref() else {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0, ui.visuals().extreme_bg_color);
            painter.text(
                rect.center() - vec2(0.0, 15.0),
                Align2::CENTER_CENTER,
                "Open a recording",
                FontId::proportional(18.0),
                ui.visuals().text_color(),
            );
            painter.text(
                rect.center() + vec2(0.0, 12.0),
                Align2::CENTER_CENTER,
                format!("Waiting for replay in {namespace}"),
                FontId::proportional(12.0),
                ui.visuals().weak_text_color(),
            );
            return;
        };
        if self.recording.as_deref() != Some(&status.instance) {
            self.recording = Some(status.instance.clone());
            self.view = None;
        }
        let (start, end, playing) = (status.start, status.end, status.playing);
        let duration = (end - start) as f64 / 1e9;
        let mut view = self.view.unwrap_or_else(|| TimeView::fit(duration));
        let connected = replay.error.is_none();
        let pointer = response.interact_pointer_pos();

        if response.hovered() {
            let (scroll, shift, pinch) = ui.input_mut(|input| {
                let scroll = input.smooth_scroll_delta;
                input.smooth_scroll_delta = egui::Vec2::ZERO;
                (scroll, input.modifiers.shift, input.zoom_delta())
            });
            if scroll != egui::Vec2::ZERO || pinch != 1.0 {
                let anchor = view.time_at(rect, response.hover_pos().unwrap_or(rect.center()).x);
                if shift {
                    view.start -=
                        f64::from(scroll.x + scroll.y) / f64::from(rect.width()) * view.span;
                } else {
                    view.start -= f64::from(scroll.x) / f64::from(rect.width()) * view.span;
                    view.zoom(
                        (-f64::from(scroll.y) * 0.005).exp() / f64::from(pinch),
                        anchor,
                        duration,
                    );
                }
                view.constrain(duration);
                self.view = Some(view);
            }
        }
        if response.dragged_by(PointerButton::Middle) {
            view.start -= f64::from(response.drag_delta().x / rect.width()) * view.span;
            view.constrain(duration);
            self.view = Some(view);
        }
        if response.double_clicked_by(PointerButton::Middle) {
            self.view = None;
            view = TimeView::fit(duration);
        }
        let primary_down =
            response.is_pointer_button_down_on() && ui.input(|input| input.pointer.primary_down());
        if connected
            && (primary_down
                || response.clicked_by(PointerButton::Primary)
                || response.dragged_by(PointerButton::Primary))
            && let Some(pointer) = pointer
        {
            response.request_focus();
            let seconds = view.time_at(rect, pointer.x).clamp(0.0, duration);
            replay.seek(start + ((seconds * 1e9).round() as u64).min(end - start));
        }
        if response.has_focus() {
            let (home, end_key, left, right, space, fit, fine) = ui.input_mut(|input| {
                let modifiers = if input.modifiers.shift {
                    Modifiers::SHIFT
                } else {
                    Modifiers::NONE
                };
                (
                    input.consume_key(Modifiers::NONE, Key::Home),
                    input.consume_key(Modifiers::NONE, Key::End),
                    input.consume_key(modifiers, Key::ArrowLeft),
                    input.consume_key(modifiers, Key::ArrowRight),
                    input.consume_key(Modifiers::NONE, Key::Space),
                    input.consume_key(Modifiers::NONE, Key::F),
                    input.modifiers.shift,
                )
            });
            if connected {
                let position = replay.position().unwrap_or(start);
                let step = (self.step_seconds * if fine { 0.1 } else { 1.0 } * 1e9).round() as u64;
                if home {
                    replay.seek(start);
                }
                if end_key {
                    replay.seek(end);
                }
                if left {
                    replay.seek(position.saturating_sub(step));
                }
                if right {
                    replay.seek(position.saturating_add(step));
                }
                if space {
                    replay.set_playing(!playing);
                }
            }
            if fit {
                self.view = None;
                view = TimeView::fit(duration);
            }
        }
        let position = replay.position().unwrap_or(start).saturating_sub(start) as f64 / 1e9;
        let status = replay.status.as_ref().expect("timeline has a recording");
        paint_timeline(
            ui,
            &response,
            view,
            duration,
            position,
            &status.recording,
            connected,
        );
        response.widget_info(|| egui::WidgetInfo::slider(connected, position, "Timeline playhead"));
        response.on_hover_cursor(if ui.input(|input| input.pointer.middle_down()) {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Crosshair
        });
    }
}

fn paint_timeline(
    ui: &Ui,
    response: &Response,
    view: TimeView,
    duration: f64,
    position: f64,
    recording: &str,
    connected: bool,
) {
    let rect = response.rect;
    let painter = ui.painter_at(rect);
    let dark = ui.visuals().dark_mode;
    let background = if dark {
        Color32::from_gray(30)
    } else {
        Color32::from_gray(205)
    };
    let range_fill = if dark {
        Color32::from_gray(39)
    } else {
        Color32::from_gray(230)
    };
    let ruler_fill = if dark {
        Color32::from_gray(48)
    } else {
        Color32::from_gray(217)
    };
    let major = if dark {
        Color32::from_gray(66)
    } else {
        Color32::from_gray(180)
    };
    let minor = if dark {
        Color32::from_gray(46)
    } else {
        Color32::from_gray(218)
    };
    let accent = if connected {
        Color32::from_rgb(75, 157, 220)
    } else {
        ui.visuals().weak_text_color()
    };
    let ruler = Rect::from_min_max(
        rect.min,
        pos2(rect.right(), (rect.top() + 28.0).min(rect.bottom())),
    );
    let body = Rect::from_min_max(pos2(rect.left(), ruler.bottom()), rect.max);
    let first = view.x_at(rect, 0.0);
    let last = view.x_at(rect, duration);
    let recorded =
        Rect::from_min_max(pos2(first, body.top()), pos2(last, body.bottom())).intersect(body);
    painter.rect_filled(rect, 0, background);
    if recorded.is_positive() {
        painter.rect_filled(recorded, 0, range_fill);
    }
    painter.rect_filled(ruler, 0, ruler_fill);

    let major_step = tick_step(view.span, rect.width());
    let minor_step = major_step / 5.0;
    let first_tick = (view.start / minor_step).floor() as i64;
    let count = (view.span / minor_step).ceil() as usize + 2;
    for index in 0..count.min(2048) {
        let tick = first_tick + index as i64;
        let time = tick as f64 * minor_step;
        let x = view.x_at(rect, time);
        if x < rect.left() || x > rect.right() {
            continue;
        }
        let is_major = tick.rem_euclid(5) == 0;
        painter.vline(
            x,
            body.y_range(),
            Stroke::new(1.0, if is_major { major } else { minor }),
        );
        painter.vline(
            x,
            (ruler.bottom() - if is_major { 7.0 } else { 3.0 })..=ruler.bottom(),
            Stroke::new(1.0, major),
        );
        if is_major {
            painter.text(
                pos2(x + 4.0, ruler.top() + 4.0),
                Align2::LEFT_TOP,
                ruler_label(time, major_step),
                FontId::monospace(11.0),
                ui.visuals().weak_text_color(),
            );
        }
    }
    painter.hline(rect.x_range(), ruler.bottom(), Stroke::new(1.0, major));
    for x in [first, last] {
        if rect.x_range().contains(x) {
            painter.vline(x, body.y_range(), Stroke::new(1.0, major));
        }
    }

    // A single recording strip: its extent is real; no invented per-topic keyframes.
    if body.height() >= 38.0 && recorded.width() > 2.0 {
        let strip = Rect::from_min_max(
            pos2(recorded.left(), body.bottom() - 27.0),
            pos2(recorded.right(), body.bottom() - 7.0),
        );
        painter.rect_filled(
            strip,
            3,
            if dark {
                Color32::from_rgb(53, 64, 73)
            } else {
                Color32::from_rgb(186, 204, 217)
            },
        );
        painter.with_clip_rect(strip.shrink(4.0)).text(
            pos2(strip.left() + 8.0, strip.center().y),
            Align2::LEFT_CENTER,
            recording,
            FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
    }
    if response.hovered()
        && !response.dragged()
        && let Some(pointer) = response.hover_pos()
    {
        painter.vline(pointer.x, body.y_range(), Stroke::new(1.0, major));
    }
    let x = view.x_at(rect, position);
    if rect.x_range().contains(x) {
        painter.vline(x, body.y_range(), Stroke::new(2.0, accent));
        let galley = painter.layout_no_wrap(
            timestamp((position * 1e9).round() as u64),
            FontId::monospace(11.0),
            Color32::WHITE,
        );
        let width = (galley.size().x + 12.0).min(rect.width());
        let badge_left = (x - width * 0.5).clamp(rect.left(), rect.right() - width);
        let badge = Rect::from_min_size(pos2(badge_left, ruler.top() + 2.0), vec2(width, 19.0));
        painter.rect_filled(badge, 3, accent);
        painter.galley(badge.center() - galley.size() * 0.5, galley, Color32::WHITE);
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(x - 5.0, ruler.bottom() - 7.0),
                pos2(x + 5.0, ruler.bottom() - 7.0),
                pos2(x, ruler.bottom()),
            ],
            accent,
            Stroke::NONE,
        ));
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            0,
            Stroke::new(1.0, accent.gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
    }
}

fn tick_step(span: f64, width: f32) -> f64 {
    let target = span * 90.0 / f64::from(width.max(1.0));
    let magnitude = 10.0_f64.powf(target.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|factor| factor * magnitude)
        .find(|step| *step >= target)
        .unwrap_or(magnitude * 10.0)
}

fn ruler_label(seconds: f64, step: f64) -> String {
    let sign = if seconds < -step * 0.001 { "−" } else { "" };
    let digits = (-step.log10().floor()).clamp(0.0, 9.0) as usize;
    let units = 10_u64.pow(digits as u32);
    let value = (seconds.abs() * units as f64).round() as u64;
    let minutes = value / units / 60;
    let seconds = value / units % 60;
    if digits > 0 {
        format!("{sign}{minutes}:{seconds:02}.{:0digits$}", value % units)
    } else {
        format!("{sign}{minutes}:{seconds:02}")
    }
}

fn transport_button(ui: &mut Ui, icon: &str, label: &str, primary: bool) -> bool {
    let mut button = Button::new(RichText::new(icon).size(18.0))
        .min_size(vec2(24.0, 22.0))
        .frame(primary);
    if primary {
        button = button.fill(ui.visuals().selection.bg_fill);
    }
    let response = ui.add(button).on_hover_text(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.clicked()
}

fn timestamp(nanos: u64) -> String {
    let millis = nanos / 1_000_000;
    let minutes = millis / 60_000;
    let seconds = millis / 1000 % 60;
    let fraction = millis % 1000;
    if minutes >= 60 {
        format!(
            "{:02}:{:02}:{seconds:02}.{fraction:03}",
            minutes / 60,
            minutes % 60
        )
    } else {
        format!("{minutes:02}:{seconds:02}.{fraction:03}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z_debug::replay::ReplayStatus;

    fn setup() -> (egui::Context, TimelinePanel, ReplaySession) {
        let context = egui::Context::default();
        egui_material_icons::initialize(&context);
        let panel = TimelinePanel {
            step_seconds: 1.0,
            view: None,
            recording: None,
        };
        let mut replay = ReplaySession::default();
        let start = 1_700_000_000_000_000_000;
        replay.status = Some(ReplayStatus {
            instance: "test".into(),
            recording: "test.mcap".into(),
            generation: 0,
            start,
            end: start + 120_000_000_000,
            position: start + 30_000_000_000,
            playing: false,
            sources: Default::default(),
        });
        (context, panel, replay)
    }

    fn frame(
        context: &egui::Context,
        panel: &mut TimelinePanel,
        replay: &mut ReplaySession,
        size: egui::Vec2,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Rect) {
        let mut canvas = Rect::NOTHING;
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Default::default(), size)),
                events,
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| panel.toolbar(ui, replay));
                canvas = ui.available_rect_before_wrap();
                panel.timeline(ui, replay, "/replay");
            },
        );
        (output, canvas)
    }

    fn pointer(pos: egui::Pos2, button: PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn full_panel_ruler_and_playhead_resize_and_scrub_on_press() {
        let (context, mut panel, mut replay) = setup();
        for size in [vec2(320.0, 140.0), vec2(900.0, 500.0)] {
            frame(&context, &mut panel, &mut replay, size, vec![]);
            let (output, canvas) = frame(&context, &mut panel, &mut replay, size, vec![]);
            assert!(canvas.width() >= size.x - 16.0 && canvas.height() >= size.y - 50.0);
            assert!(
                canvas.right() <= size.x + 1.0 && canvas.bottom() <= size.y + 1.0,
                "timeline must fit its allotted panel, including narrow headers"
            );
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::LineSegment { points, stroke } if stroke.width == 2.0 && (points[1].y - canvas.bottom()).abs() < 1.0)),
                "playhead must extend to the bottom of the panel");
            assert!(
                output.shapes.len() < 500,
                "drawing cost must depend on pixels, not recording length"
            );
            let x = TimeView::fit(120.0).x_at(canvas, 90.0);
            let pos = pos2(x, canvas.center().y);
            frame(
                &context,
                &mut panel,
                &mut replay,
                size,
                vec![
                    egui::Event::PointerMoved(pos),
                    pointer(pos, PointerButton::Primary, true),
                ],
            );
            let start = replay.status.as_ref().unwrap().start;
            assert!(
                replay.position().unwrap().abs_diff(start + 90_000_000_000) < 100_000,
                "scrub starts on press, not after drag threshold or release"
            );
            let outside = pos2(canvas.right() + 100.0, canvas.center().y);
            frame(
                &context,
                &mut panel,
                &mut replay,
                size,
                vec![egui::Event::PointerMoved(outside)],
            );
            assert_eq!(replay.position(), Some(start + 120_000_000_000));
            frame(
                &context,
                &mut panel,
                &mut replay,
                size,
                vec![pointer(outside, PointerButton::Primary, false)],
            );
        }
    }

    #[test]
    fn zoom_is_pointer_anchored_and_pan_does_not_seek() {
        let (context, mut panel, mut replay) = setup();
        let size = vec2(800.0, 300.0);
        frame(&context, &mut panel, &mut replay, size, vec![]);
        let (_, canvas) = frame(&context, &mut panel, &mut replay, size, vec![]);
        let pos = pos2(canvas.center().x, canvas.center().y);
        let before = TimeView::fit(120.0).time_at(canvas, pos.x);
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![egui::Event::PointerMoved(pos), egui::Event::Zoom(2.0)],
        );
        let zoomed = panel.view.expect("zoom changes the viewport");
        assert!((zoomed.span - 66.0).abs() < 0.001);
        assert!((zoomed.time_at(canvas, pos.x) - before).abs() < 0.001);
        let original_position = replay.position();
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![pointer(pos, PointerButton::Middle, true)],
        );
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![egui::Event::PointerMoved(pos + vec2(80.0, 0.0))],
        );
        assert!(panel.view.unwrap().start < zoomed.start);
        assert_eq!(replay.position(), original_position);
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![pointer(pos + vec2(80.0, 0.0), PointerButton::Middle, false)],
        );
        replay.status.as_mut().unwrap().instance = "another recording".into();
        frame(&context, &mut panel, &mut replay, size, vec![]);
        assert!(panel.view.is_none(), "a new recording must start framed");
    }

    #[test]
    fn focused_timeline_supports_keyboard_and_small_or_empty_ranges() {
        let (context, mut panel, mut replay) = setup();
        let size = vec2(600.0, 200.0);
        frame(&context, &mut panel, &mut replay, size, vec![]);
        let (_, canvas) = frame(&context, &mut panel, &mut replay, size, vec![]);
        let pos = canvas.center();
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![
                egui::Event::PointerMoved(pos),
                pointer(pos, PointerButton::Primary, true),
                pointer(pos, PointerButton::Primary, false),
            ],
        );
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![key(Key::Home)],
        );
        assert_eq!(replay.position(), replay.status.as_ref().map(|s| s.start));
        frame(
            &context,
            &mut panel,
            &mut replay,
            size,
            vec![key(Key::ArrowRight)],
        );
        assert_eq!(
            replay.position(),
            replay.status.as_ref().map(|s| s.start + 1_000_000_000)
        );
        panel.view = Some(TimeView {
            start: 20.0,
            span: 5.0,
        });
        frame(&context, &mut panel, &mut replay, size, vec![key(Key::F)]);
        assert!(panel.view.is_none());
        let mut view = TimeView::fit(0.0);
        view.zoom(0.000_001, 0.0, 0.0);
        assert!(view.span > 0.0 && view.start.is_finite());
        assert_eq!(ruler_label(0.0002, 0.0001), "0:00.0002");
        assert_eq!(ruler_label(120.0, 10.0), "2:00");
        assert_eq!(timestamp(3_661_234_000_000), "01:01:01.234");
        assert_eq!(panel.save(), json!({"step_seconds": 1.0}));
    }
}
