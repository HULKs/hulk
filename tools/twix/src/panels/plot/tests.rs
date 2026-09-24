use super::*;
use std::sync::Arc;

use crate::backend::RobotBackend;
use eframe::egui::{CentralPanel, Context, Event, PointerButton, Pos2, RawInput, Rect, vec2};
use serde_json::json;

#[test]
fn settings_restore_without_an_entered_runtime_and_exclude_transient_state() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let backend = Arc::new(
        runtime
            .block_on(RobotBackend::new(
                runtime.handle().clone(),
                None,
                "/".into(),
            ))
            .unwrap(),
    );
    let saved = json!({
        "history_seconds": 12.5,
        "lines": [
            { "topic": "detections", "field_path": "inner[2].bounding_box.confidence", "color": COLORS[2], "visible": false },
            { "topic": "speed", "field_path": "", "color": COLORS[1], "visible": true }
        ]
    });
    let mut panel = PlotPanel::new(PanelCreationContext {
        backend,
        value: Some(&saved),
        egui_context: Context::default(),
    });
    panel.paused = true;
    assert_eq!(panel.save(), saved);
    assert_eq!(
        crate::PanelKind::from_storage_id("plot").unwrap(),
        crate::PanelKind::PlotPanel
    );
    assert!(crate::PanelKind::registered().contains(&crate::PanelKind::PlotPanel));
    assert_eq!(valid_history_seconds(f64::NAN), 30.0);
    assert_eq!(valid_history_seconds(-1.0), 1.0);
    assert_eq!(valid_history_seconds(1000.0), 600.0);
}

fn frame(
    panel: &mut PlotPanel,
    backend: &Arc<RobotBackend>,
    context: &Context,
    width: f32,
    events: Vec<Event>,
) -> eframe::egui::FullOutput {
    context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 600.0))),
            events,
            ..Default::default()
        },
        |ui| {
            CentralPanel::default().show(ui, |ui| {
                let available = ui.available_width();
                let response = ui.scope(|ui| {
                    panel.header_ui(
                        ui,
                        PanelUiContext {
                            backend,
                            egui_context: context,
                        },
                    );
                    panel.ui(
                        ui,
                        PanelUiContext {
                            backend,
                            egui_context: context,
                        },
                    );
                });
                assert!(
                    response.response.rect.width() <= available + 1.0,
                    "width={width}, rect={:?}",
                    response.response.rect
                );
            });
        },
    )
}

fn click_button(output: &eframe::egui::FullOutput, label: &str) -> Vec<Event> {
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            eframe::egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + vec2(5.0, 5.0))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing button {label}"));
    vec![
        Event::PointerMoved(position),
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        },
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plot_controls_fit_split_panes_and_pause_resume_and_add_lines() {
    let backend = Arc::new(
        RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
            .await
            .unwrap(),
    );
    for width in [240.0, 800.0] {
        let context = Context::default();
        let mut panel = PlotPanel::new(PanelCreationContext {
            backend: Arc::clone(&backend),
            value: None,
            egui_context: context.clone(),
        });
        let output = frame(&mut panel, &backend, &context, width, vec![]);
        let output = frame(
            &mut panel,
            &backend,
            &context,
            width,
            click_button(&output, "Add line"),
        );
        assert_eq!(panel.lines.len(), 2);
        let ids: Vec<_> = panel.lines.iter().map(|line| line.id).collect();
        assert_ne!(ids[0], ids[1]);
        frame(
            &mut panel,
            &backend,
            &context,
            width,
            click_button(&output, "Pause"),
        );
        assert!(panel.paused);
        let output = frame(&mut panel, &backend, &context, width, vec![]);
        frame(
            &mut panel,
            &backend,
            &context,
            width,
            click_button(&output, "Resume"),
        );
        assert!(!panel.paused);
    }
}
