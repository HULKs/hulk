use super::{paint::ruler_label, *};
use crate::{
    configuration::{
        Configuration,
        keybind_plugin::{self, KeybindSystem},
        keys::{Keybinds, parse_shortcut},
    },
    replay::ReplayStatus,
};
use eframe::egui::{
    Context, Event, FullOutput, Key, Modifiers, OutputCommand, Pos2, RawInput, Shape, TextEdit,
    UiBuilder, pos2,
};
use std::sync::Arc;

struct TimelineTest {
    context: Context,
    panel: TimelinePanel,
    replay: ReplaySession,
    size: Vec2,
}

impl TimelineTest {
    fn new(size: Vec2) -> Self {
        let context = Context::default();
        context.set_keybinds(Arc::new(Configuration::default().keys));
        keybind_plugin::register(&context);
        egui_material_icons::initialize(&context);
        let panel = TimelinePanel {
            step_seconds: 1.0,
            view: None,
            recording: None,
            restore_focus: None,
        };
        let mut replay = ReplaySession::default();
        let start = 1_700_000_000_000_000_000;
        replay.status = Some(ReplayStatus {
            instance: "test".into(),
            recording: "test.mcap".into(),
            start,
            end: start + 120_000_000_000,
            position: start + 30_000_000_000,
            playing: false,
        });
        Self {
            context,
            panel,
            replay,
            size,
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> (FullOutput, Rect) {
        let Self {
            context,
            panel,
            replay,
            size,
        } = self;
        let mut canvas = Rect::NOTHING;
        let output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Default::default(), *size)),
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
}

fn pointer(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn click(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        pointer(pos, PointerButton::Primary, true),
        pointer(pos, PointerButton::Primary, false),
    ]
}

fn key_event(key: Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn press(key: Key) -> Vec<Event> {
    vec![key_event(key, true), key_event(key, false)]
}

fn text_rect(output: &FullOutput, label: &str) -> Rect {
    let text = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.text() == label => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "missing text: {label}; rendered: {:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            )
        });
    Rect::from_min_size(text.pos, text.galley.size())
}

#[test]
fn toolbar_focus_supports_space_and_fixed_two_millisecond_steps() {
    let mut test = TimelineTest::new(vec2(600.0, 250.0));
    test.frame(vec![]);
    let (output, _) = test.frame(vec![]);
    let forward = text_rect(&output, icons::ICON_FAST_FORWARD.codepoint).center();
    test.frame(click(forward));
    let position = test.replay.position().unwrap();
    test.frame(press(Key::Space));
    assert!(test.replay.status.as_ref().unwrap().playing);
    assert_eq!(
        test.replay.position(),
        Some(position),
        "Space must not activate the focused step button"
    );
    test.frame(vec![key_event(Key::Space, true)]);
    assert!(!test.replay.status.as_ref().unwrap().playing);
    test.frame(vec![key_event(Key::Space, true)]);
    assert!(
        !test.replay.status.as_ref().unwrap().playing,
        "auto-repeat must not toggle playback"
    );
    test.frame(vec![key_event(Key::Space, false)]);
    test.panel.step_seconds = 7.25;
    test.replay.set_playing(true);
    test.frame(press(Key::Comma));
    assert_eq!(test.replay.position(), Some(position - 2_000_000));
    assert!(!test.replay.status.as_ref().unwrap().playing);
    test.frame(press(Key::Period));
    assert_eq!(test.replay.position(), Some(position));
    test.frame(press(Key::Home));
    test.frame(press(Key::Comma));
    assert_eq!(
        test.replay.position(),
        test.replay.status.as_ref().map(|status| status.start)
    );
    test.frame(press(Key::End));
    test.frame(press(Key::Period));
    assert_eq!(
        test.replay.position(),
        test.replay.status.as_ref().map(|status| status.end)
    );
}

#[test]
fn pane_focus_supports_shortcuts_without_stealing_text_input() {
    let TimelineTest {
        context,
        mut panel,
        mut replay,
        ..
    } = TimelineTest::new(vec2(600.0, 300.0));
    let pane = Id::new("timeline-pane");
    let editor = Id::new("other-panel-editor");
    let mut text = String::new();
    let mut render = |events| {
        let _ = context.run_ui(
            RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Default::default(), vec2(600.0, 300.0))),
                ..Default::default()
            },
            |ui| {
                ui.add(TextEdit::singleline(&mut text).id(editor));
                ui.scope_builder(
                    UiBuilder::new()
                        .id(pane)
                        .sense(Sense::focusable_noninteractive()),
                    |ui| {
                        ui.horizontal(|ui| panel.toolbar(ui, &mut replay));
                        panel.timeline(ui, &mut replay, "/replay");
                    },
                );
            },
        );
        replay.status.as_ref().unwrap().playing
    };
    render(vec![]);
    context.memory_mut(|memory| memory.request_focus(pane));
    assert!(render(press(Key::Space)));
    context.memory_mut(|memory| memory.request_focus(editor));
    let mut events = press(Key::Space);
    events.extend(press(Key::Comma));
    events.extend(press(Key::Period));
    events.push(Event::Text(",. ".into()));
    render(events);
    assert!(replay.status.as_ref().unwrap().playing);
    assert_eq!(
        replay.position(),
        replay
            .status
            .as_ref()
            .map(|status| status.start + 30_000_000_000)
    );
    assert_eq!(text, ",. ");
}

#[test]
fn popup_edits_stay_open_and_copied_seconds_round_trip() {
    let mut test = TimelineTest::new(vec2(600.0, 350.0));
    let start = test.replay.status.as_ref().unwrap().start;
    let elapsed = 30_123_456_789;
    test.replay.seek(start + elapsed);
    test.frame(vec![]);
    let (output, _) = test.frame(vec![]);
    let options = text_rect(&output, "⋯").center();
    test.frame(click(options));
    let (output, _) = test.frame(vec![]);
    assert!(Popup::is_any_open(&test.context));
    assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,Shape::Text(text) if text.galley.text().starts_with("Drag:") || text.galley.text().starts_with("Shift + wheel:"))));
    let copy = text_rect(&output, "Copy timestamp").center();
    let (output, _) = test.frame(click(copy));
    let copied = output
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("clipboard text");
    assert_eq!(copied, "30.123456789");
    assert!(!Popup::is_any_open(&test.context));
    test.frame(press(Key::Space));
    assert!(
        test.replay.status.as_ref().unwrap().playing,
        "closing a popup action retains timeline shortcuts"
    );
    test.replay.seek(start);
    test.replay.set_playing(true);
    test.frame(vec![]);
    test.frame(click(options));
    let (output, _) = test.frame(vec![]);
    let label = text_rect(&output, "Jump to");
    let jump = pos2(label.right() + 20.0, label.center().y);
    test.frame(click(jump));
    assert!(Popup::is_any_open(&test.context));
    assert!(
        !test.replay.status.as_ref().unwrap().playing,
        "editing the playhead pauses its moving value"
    );
    test.frame(vec![]);
    let (output, _) = test.frame(vec![Event::Paste(copied)]);
    assert_eq!(test.replay.position(), Some(start + elapsed));
    assert!(Popup::is_any_open(&test.context));
    let label = text_rect(&output, "Step");
    test.frame(click(pos2(label.right() + 20.0, label.center().y)));
    test.frame(vec![]);
    test.frame(vec![Event::Paste("0.125".into())]);
    assert_eq!(test.panel.step_seconds, 0.125);
    assert!(Popup::is_any_open(&test.context));
    let mut keys = press(Key::Space);
    keys.extend(press(Key::Comma));
    keys.extend(press(Key::Period));
    test.frame(keys);
    assert_eq!(test.replay.position(), Some(start + elapsed));
    assert!(!test.replay.status.as_ref().unwrap().playing);
    test.frame(press(Key::Escape));
    assert!(!Popup::is_any_open(&test.context));
    test.frame(press(Key::Space));
    assert!(
        test.replay.status.as_ref().unwrap().playing,
        "Escape restores timeline shortcut focus"
    );
}

#[test]
fn full_panel_ruler_and_playhead_resize_and_scrub_on_press() {
    let mut test = TimelineTest::new(vec2(320.0, 140.0));
    for size in [vec2(320.0, 140.0), vec2(900.0, 500.0)] {
        test.size = size;
        test.frame(vec![]);
        let (output, canvas) = test.frame(vec![]);
        assert!(canvas.width() >= size.x - 16.0 && canvas.height() >= size.y - 50.0);
        assert!(
            canvas.right() <= size.x + 1.0 && canvas.bottom() <= size.y + 1.0,
            "timeline must fit its allotted panel, including narrow headers"
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            Shape::LineSegment { points, stroke } if stroke.width == 2.0 && (points[1].y - canvas.bottom()).abs() < 1.0)),
            "playhead must extend to the bottom of the panel");
        assert!(
            output.shapes.len() < 500,
            "drawing cost must depend on pixels, not recording length"
        );
        let x = TimeView::fit(120.0).x_at(canvas, 90.0);
        let pos = pos2(x, canvas.center().y);
        test.frame(vec![
            Event::PointerMoved(pos),
            pointer(pos, PointerButton::Primary, true),
        ]);
        let start = test.replay.status.as_ref().unwrap().start;
        assert!(
            test.replay
                .position()
                .unwrap()
                .abs_diff(start + 90_000_000_000)
                < 100_000,
            "scrub starts on press, not after drag threshold or release"
        );
        let outside = pos2(canvas.right() + 100.0, canvas.center().y);
        test.frame(vec![Event::PointerMoved(outside)]);
        assert_eq!(test.replay.position(), Some(start + 120_000_000_000));
        let release = pos2(TimeView::fit(120.0).x_at(canvas, 45.0), canvas.center().y);
        test.frame(vec![
            Event::PointerMoved(release),
            pointer(release, PointerButton::Primary, false),
        ]);
        assert!(
            test.replay
                .position()
                .unwrap()
                .abs_diff(start + 45_000_000_000)
                < 100_000,
            "release must commit its final pointer position immediately"
        );
    }
}

#[test]
fn zoom_is_pointer_anchored_and_pan_does_not_seek() {
    let mut test = TimelineTest::new(vec2(800.0, 300.0));
    test.frame(vec![]);
    let (_, canvas) = test.frame(vec![]);
    let pos = pos2(canvas.center().x, canvas.center().y);
    let before = TimeView::fit(120.0).time_at(canvas, pos.x);
    test.frame(vec![Event::PointerMoved(pos), Event::Zoom(2.0)]);
    let zoomed = test.panel.view.expect("zoom changes the viewport");
    assert!((zoomed.span - 66.0).abs() < 0.001);
    assert!((zoomed.time_at(canvas, pos.x) - before).abs() < 0.001);
    let original_position = test.replay.position();
    test.frame(vec![pointer(pos, PointerButton::Middle, true)]);
    test.frame(vec![Event::PointerMoved(pos + vec2(80.0, 0.0))]);
    assert!(test.panel.view.unwrap().start < zoomed.start);
    assert_eq!(test.replay.position(), original_position);
    test.frame(vec![pointer(
        pos + vec2(80.0, 0.0),
        PointerButton::Middle,
        false,
    )]);
    test.replay.status.as_mut().unwrap().instance = "another recording".into();
    test.frame(vec![]);
    assert!(
        test.panel.view.is_none(),
        "a new recording must start framed"
    );
}

#[test]
fn focused_timeline_supports_keyboard() {
    let mut test = TimelineTest::new(vec2(600.0, 200.0));
    test.frame(vec![]);
    let (_, canvas) = test.frame(vec![]);
    test.frame(click(canvas.center()));
    test.frame(vec![key_event(Key::Home, true)]);
    assert_eq!(
        test.replay.position(),
        test.replay.status.as_ref().map(|s| s.start)
    );
    test.frame(vec![key_event(Key::ArrowRight, true)]);
    assert_eq!(
        test.replay.position(),
        test.replay.status.as_ref().map(|s| s.start + 1_000_000_000)
    );
    test.panel.view = Some(TimeView {
        start: 20.0,
        span: 5.0,
    });
    test.frame(vec![key_event(Key::F, true)]);
    assert!(test.panel.view.is_none());
    test.frame(press(Key::Space));
    assert!(test.replay.status.as_ref().unwrap().playing);
    test.frame(press(Key::Space));
    assert!(!test.replay.status.as_ref().unwrap().playing);
}

#[test]
fn configured_bindings_replace_defaults_and_buttons_do_not_execute_twice() {
    let mut test = TimelineTest::new(vec2(600.0, 250.0));
    test.frame(vec![]);
    let (output, _) = test.frame(vec![]);
    test.frame(click(
        text_rect(&output, icons::ICON_FAST_FORWARD.codepoint).center(),
    ));
    let position = test.replay.position().unwrap();
    let bindings: Keybinds = toml::from_str(
        r#"
        Space = "no_op"
        p = "timeline_toggle_playback"
        Enter = "timeline_step_backward"
    "#,
    )
    .unwrap();
    test.context.set_keybinds(Arc::new(bindings));
    assert_eq!(
        test.context
            .keybind_label(KeybindAction::TimelineTogglePlayback),
        test.context.format_shortcut(&parse_shortcut("p").unwrap())
    );
    test.frame(press(Key::Space));
    assert!(!test.replay.status.as_ref().unwrap().playing);
    assert_eq!(test.replay.position(), Some(position));
    test.frame(press(Key::P));
    assert!(test.replay.status.as_ref().unwrap().playing);
    test.frame(press(Key::Enter));
    assert!(!test.replay.status.as_ref().unwrap().playing);
    assert_eq!(test.replay.position(), Some(position - 1_000_000_000));
}

#[test]
fn configured_fine_steps_are_independent_actions() {
    let mut test = TimelineTest::new(vec2(600.0, 200.0));
    test.frame(vec![]);
    let (_, canvas) = test.frame(vec![]);
    test.frame(click(canvas.center()));
    test.panel.step_seconds = 7.25;
    let position = test.replay.position().unwrap();
    test.frame(
        [true, false]
            .into_iter()
            .map(|pressed| Event::Key {
                key: Key::ArrowRight,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            })
            .collect(),
    );
    assert_eq!(test.replay.position(), Some(position + 725_000_000));
}

#[test]
fn empty_views_and_timestamp_labels() {
    let mut view = TimeView::fit(0.0);
    view.zoom(0.000_001, 0.0, 0.0);
    assert!(view.span > 0.0 && view.start.is_finite());
    assert_eq!(ruler_label(0.0002, 0.0001), "0:00.0002");
    assert_eq!(ruler_label(120.0, 10.0), "2:00");
    assert_eq!(timestamp(3_661_234_000_000), "01:01:01.234");
}
