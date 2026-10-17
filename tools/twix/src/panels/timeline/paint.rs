use eframe::egui::{
    Align2, Color32, FontId, Rect, Response, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::{TimeView, timestamp};

pub(super) fn paint_empty(ui: &Ui, rect: Rect, namespace: &str) {
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
}

pub(super) fn paint_timeline(
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
    let gray = |dark, light| Color32::from_gray(if ui.visuals().dark_mode { dark } else { light });
    let major = gray(66, 180);
    let minor = gray(46, 218);
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
    painter.rect_filled(rect, 0, gray(30, 205));
    if recorded.is_positive() {
        painter.rect_filled(recorded, 0, gray(39, 230));
    }
    painter.rect_filled(ruler, 0, gray(48, 217));
    paint_ruler(ui, view, rect, ruler, body, major, minor);
    for x in [first, last] {
        if rect.x_range().contains(x) {
            painter.vline(x, body.y_range(), Stroke::new(1.0, major));
        }
    }
    paint_recording_strip(ui, body, recorded, recording);
    if response.hovered()
        && !response.dragged()
        && let Some(pointer) = response.hover_pos()
    {
        painter.vline(pointer.x, body.y_range(), Stroke::new(1.0, major));
    }
    paint_playhead(
        ui,
        rect,
        ruler,
        body,
        view.x_at(rect, position),
        position,
        accent,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            0,
            Stroke::new(1.0, accent.gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
    }
}

fn paint_ruler(
    ui: &Ui,
    view: TimeView,
    rect: Rect,
    ruler: Rect,
    body: Rect,
    major: Color32,
    minor: Color32,
) {
    let painter = ui.painter_at(rect);
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
}

fn paint_recording_strip(ui: &Ui, body: Rect, recorded: Rect, recording: &str) {
    if body.height() < 38.0 || recorded.width() <= 2.0 {
        return;
    }
    let painter = ui.painter_at(body);
    let strip = Rect::from_min_max(
        pos2(recorded.left(), body.bottom() - 27.0),
        pos2(recorded.right(), body.bottom() - 7.0),
    );
    let color = if ui.visuals().dark_mode {
        Color32::from_rgb(53, 64, 73)
    } else {
        Color32::from_rgb(186, 204, 217)
    };
    painter.rect_filled(strip, 3, color);
    painter.with_clip_rect(strip.shrink(4.0)).text(
        pos2(strip.left() + 8.0, strip.center().y),
        Align2::LEFT_CENTER,
        recording,
        FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
}

fn paint_playhead(
    ui: &Ui,
    rect: Rect,
    ruler: Rect,
    body: Rect,
    x: f32,
    position: f64,
    accent: Color32,
) {
    if !rect.x_range().contains(x) {
        return;
    }
    let painter = ui.painter_at(rect);
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
    painter.add(Shape::convex_polygon(
        vec![
            pos2(x - 5.0, ruler.bottom() - 7.0),
            pos2(x + 5.0, ruler.bottom() - 7.0),
            pos2(x, ruler.bottom()),
        ],
        accent,
        Stroke::NONE,
    ));
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

pub(super) fn ruler_label(seconds: f64, step: f64) -> String {
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
