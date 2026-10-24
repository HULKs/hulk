use eframe::egui::{
    Align, Button, CursorIcon, DragValue, Id, Key, Layout, PointerButton, Popup,
    PopupCloseBehavior, Rect, Response, RichText, Role, Sense, Ui, Vec2, WidgetInfo, vec2,
};
use egui_material_icons::icons;
use serde_json::{Value, json};

use crate::{
    configuration::{keybind_plugin::KeybindSystem, keys::KeybindAction},
    panel::{Panel, PanelCreationContext, PanelUiContext},
    replay::ReplaySession,
};
use paint::{paint_empty, paint_timeline};

mod paint;
#[cfg(test)]
mod tests;

pub struct TimelinePanel {
    step_seconds: f64,
    view: Option<TimeView>,
    recording: Option<String>,
    restore_focus: Option<Id>,
}

/// Seconds relative to the recording start keep Unix timestamps out of f32 math.
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
            restore_focus: None,
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
    fn keyboard(&mut self, ui: &Ui, replay: &mut ReplaySession) {
        if Popup::is_any_open(ui.ctx()) || !timeline_has_focus(ui) || replay.status.is_none() {
            return;
        }
        for action in ui.ctx().consume_timeline_actions() {
            self.apply_action(action, replay);
        }
    }

    fn apply_action(&mut self, action: KeybindAction, replay: &mut ReplaySession) {
        if action == KeybindAction::TimelineFrameAll {
            self.view = None;
            return;
        }
        if replay.error.is_some() {
            return;
        }
        let Some(status) = &replay.status else { return };
        let (start, end, playing) = (status.start, status.end, status.playing);
        let position = replay.position().unwrap_or(start);
        let step = (self.step_seconds * 1e9).round() as u64;
        let fine_step = (self.step_seconds * 0.1 * 1e9).round() as u64;
        match action {
            KeybindAction::TimelineStart => replay.seek(start),
            KeybindAction::TimelineEnd => replay.seek(end),
            KeybindAction::TimelineTogglePlayback => replay.set_playing(!playing),
            KeybindAction::TimelineStepBackward => replay.seek(position.saturating_sub(step)),
            KeybindAction::TimelineStepForward => replay.seek(position.saturating_add(step)),
            KeybindAction::TimelineFineStepBackward => {
                replay.seek(position.saturating_sub(fine_step))
            }
            KeybindAction::TimelineFineStepForward => {
                replay.seek(position.saturating_add(fine_step))
            }
            KeybindAction::TimelineNudgeBackward => replay.seek(position.saturating_sub(2_000_000)),
            KeybindAction::TimelineNudgeForward => replay.seek(position.saturating_add(2_000_000)),
            _ => {}
        }
    }

    fn restore_keyboard_focus(&mut self, ui: &Ui) {
        // Closing a popup can clear focus later in the pass; restore it next pass.
        if let Some(id) = self.restore_focus.take()
            && ui.memory(|memory| memory.focused().is_none())
            && !ui.input(|input| input.pointer.any_pressed() || input.key_pressed(Key::Tab))
        {
            ui.memory_mut(|memory| memory.request_focus(id));
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, replay: &mut ReplaySession) {
        self.restore_keyboard_focus(ui);
        // Consume configured shortcuts before a focused button can activate on Space.
        self.keyboard(ui, replay);
        let Some(status) = replay.status.as_ref() else {
            ui.weak("Timeline");
            if replay.error.is_some() {
                retry_button(ui, replay);
            } else {
                ui.spinner();
            }
            return;
        };
        let (start, end) = (status.start, status.end);
        let position = replay.position().unwrap_or(status.position);
        let width = ui.available_width();
        ui.spacing_mut().item_spacing.x = 3.0;
        self.transport_controls(ui, replay);
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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            self.keyboard(ui, replay);
            self.options_menu(ui, replay);
            self.action_button(
                ui,
                replay,
                KeybindAction::TimelineFrameAll,
                icons::ICON_FIT_SCREEN.codepoint,
                "Frame all",
            );
            if replay.error.is_some() {
                retry_button(ui, replay);
            } else if width >= 580.0 {
                let playing = replay.status.as_ref().is_some_and(|status| status.playing);
                ui.weak(if playing { "Playing" } else { "Paused" });
            }
        });
    }

    fn transport_controls(&mut self, ui: &mut Ui, replay: &mut ReplaySession) {
        let playing = replay.status.as_ref().is_some_and(|status| status.playing);
        let (icon, label) = if playing {
            (icons::ICON_PAUSE.codepoint, "Pause")
        } else {
            (icons::ICON_PLAY_ARROW.codepoint, "Play")
        };
        for (action, icon, label) in [
            (
                KeybindAction::TimelineStart,
                icons::ICON_SKIP_PREVIOUS.codepoint,
                "Start",
            ),
            (
                KeybindAction::TimelineStepBackward,
                icons::ICON_FAST_REWIND.codepoint,
                "Step backward",
            ),
            (KeybindAction::TimelineTogglePlayback, icon, label),
            (
                KeybindAction::TimelineStepForward,
                icons::ICON_FAST_FORWARD.codepoint,
                "Step forward",
            ),
            (
                KeybindAction::TimelineEnd,
                icons::ICON_SKIP_NEXT.codepoint,
                "End",
            ),
        ] {
            self.action_button(ui, replay, action, icon, label);
        }
    }

    fn action_button(
        &mut self,
        ui: &mut Ui,
        replay: &mut ReplaySession,
        action: KeybindAction,
        icon: &str,
        label: &str,
    ) {
        let primary = action == KeybindAction::TimelineTogglePlayback;
        let enabled = action == KeybindAction::TimelineFrameAll
            || (replay.error.is_none()
                && replay.status.as_ref().is_some_and(|status| {
                    !primary || status.playing || status.position < status.end
                }));
        let mut button = Button::new(RichText::new(icon).size(18.0))
            .min_size(vec2(24.0, 22.0))
            .frame(primary);
        if primary {
            button = button.fill(ui.visuals().selection.bg_fill);
        }
        let shortcut = ui.ctx().keybind_label(action);
        let tooltip = if shortcut.is_empty() {
            label.to_owned()
        } else {
            format!("{label} ({shortcut})")
        };
        let response = ui.add_enabled(enabled, button).on_hover_text(tooltip);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
        if response.clicked() {
            response.request_focus();
            self.apply_action(action, replay);
        }
    }

    fn options_menu(&mut self, ui: &mut Ui, replay: &mut ReplaySession) {
        let options = ui.button("⋯").on_hover_text("Timeline options");
        if options.clicked() {
            options.request_focus();
        }
        let popup = Popup::menu(&options).close_behavior(PopupCloseBehavior::CloseOnClickOutside);
        let escape = popup.is_open() && ui.input(|input| input.key_pressed(Key::Escape));
        let mut restore_focus = options.clicked();
        popup.show(|ui| {
            ui.add_enabled_ui(replay.error.is_none(), |ui| self.jump_editor(ui, replay));
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
            if ui.button("Copy timestamp").clicked() {
                let status = replay.status.as_ref().expect("timeline has a recording");
                ui.ctx().copy_text(seconds_timestamp(
                    status.position.saturating_sub(status.start),
                ));
                ui.close();
                restore_focus = true;
            }
        });
        if (restore_focus || escape)
            && !Popup::is_id_open(ui.ctx(), Popup::default_response_id(&options))
        {
            self.restore_focus = Some(options.id);
        }
    }

    fn jump_editor(&mut self, ui: &mut Ui, replay: &mut ReplaySession) {
        let Some(status) = replay.status.as_ref() else {
            return;
        };
        let (start, end) = (status.start, status.end);
        let mut seconds = status.position.saturating_sub(start) as f64 / 1e9;
        ui.horizontal(|ui| {
            ui.label("Jump to");
            let response = ui.add(
                DragValue::new(&mut seconds)
                    .range(0.0..=(end - start) as f64 / 1e9)
                    .speed(0.1)
                    .max_decimals(9)
                    .suffix(" s"),
            );
            if response.gained_focus() {
                replay.set_playing(false);
            }
            if response.changed() {
                replay.seek(start + ((seconds * 1e9).round() as u64).min(end - start));
            }
        });
    }

    fn timeline(&mut self, ui: &mut Ui, replay: &mut ReplaySession, namespace: &str) {
        self.keyboard(ui, replay);
        let size = ui.available_size().max(vec2(1.0, 1.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let Some(status) = replay.status.as_ref() else {
            paint_empty(ui, rect, namespace);
            return;
        };
        if self.recording.as_deref() != Some(&status.instance) {
            self.recording = Some(status.instance.clone());
            self.view = None;
        }
        let (start, end) = (status.start, status.end);
        let duration = (end - start) as f64 / 1e9;
        let mut view = self.view.unwrap_or_else(|| TimeView::fit(duration));
        self.navigate(ui, &response, &mut view, duration);
        if replay.error.is_none() {
            scrub(ui, &response, view, replay);
        }
        let position = replay.position().unwrap_or(start).saturating_sub(start) as f64 / 1e9;
        let status = replay.status.as_ref().expect("timeline has a recording");
        let connected = replay.error.is_none();
        paint_timeline(
            ui,
            &response,
            view,
            duration,
            position,
            &status.recording,
            connected,
        );
        response.widget_info(|| WidgetInfo::slider(connected, position, "Timeline playhead"));
        response.on_hover_cursor(if ui.input(|input| input.pointer.middle_down()) {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Crosshair
        });
    }

    fn navigate(&mut self, ui: &Ui, response: &Response, view: &mut TimeView, duration: f64) {
        let rect = response.rect;
        if response.hovered() {
            let (scroll, shift, pinch) = ui.input_mut(|input| {
                let scroll = input.smooth_scroll_delta;
                input.smooth_scroll_delta = Vec2::ZERO;
                (scroll, input.modifiers.shift, input.zoom_delta())
            });
            if scroll != Vec2::ZERO || pinch != 1.0 {
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
                self.view = Some(*view);
            }
        }
        if response.dragged_by(PointerButton::Middle) {
            view.start -= f64::from(response.drag_delta().x / rect.width()) * view.span;
            view.constrain(duration);
            self.view = Some(*view);
        }
        if response.double_clicked_by(PointerButton::Middle) {
            self.view = None;
            *view = TimeView::fit(duration);
        }
    }
}

fn scrub(ui: &Ui, response: &Response, view: TimeView, replay: &mut ReplaySession) {
    let primary_down =
        response.is_pointer_button_down_on() && ui.input(|input| input.pointer.primary_down());
    if !(primary_down
        || response.clicked_by(PointerButton::Primary)
        || response.drag_stopped_by(PointerButton::Primary)
        || response.dragged_by(PointerButton::Primary))
    {
        return;
    }
    let Some(pointer) = response.interact_pointer_pos() else {
        return;
    };
    response.request_focus();
    let status = replay.status.as_ref().expect("timeline has a recording");
    let duration = (status.end - status.start) as f64 / 1e9;
    let seconds = view.time_at(response.rect, pointer.x).clamp(0.0, duration);
    let position = status.start + ((seconds * 1e9).round() as u64).min(status.end - status.start);
    if ui.input(|input| input.pointer.primary_down()) {
        replay.scrub_to(position);
    } else {
        replay.seek(position);
    }
}

/// Pane, canvas and toolbar focus qualify; sibling editors do not.
fn timeline_has_focus(ui: &Ui) -> bool {
    if !ui.input(|input| input.focused) {
        return false;
    }
    let Some(focused) = ui.memory(|memory| memory.focused()) else {
        return false;
    };
    ui.stack()
        .iter()
        .any(|scope| scope.scope_id == focused || scope.unique_id == focused)
        || ui
            .ctx()
            .read_response(focused)
            .is_some_and(|response| response.parent_id() == ui.unique_id())
}

fn retry_button(ui: &mut Ui, replay: &mut ReplaySession) {
    if ui
        .small_button("Retry")
        .on_hover_text(replay.error.as_deref().unwrap_or_default())
        .clicked()
    {
        replay.retry();
    }
}

fn seconds_timestamp(nanos: u64) -> String {
    format!("{}.{:09}", nanos / 1_000_000_000, nanos % 1_000_000_000)
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
