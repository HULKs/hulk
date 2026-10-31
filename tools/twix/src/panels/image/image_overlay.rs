use super::{
    inputs::{OverlayInput, OverlayInputs},
    overlays::{HorizonOverlay, ObjectDetectionOverlay, PoseDetectionOverlay},
};
use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{
    PopupCloseBehavior, Ui,
    containers::menu::{MenuButton, MenuConfig},
};
use ros_z::time::Time;
use ros_z_debug::query::plan::{Frame, InputSpec, Source};
use serde_json::{Value, json};
use std::ops::Range;
use twix_visualization::twix_painter::TwixPainter;

pub(super) trait ImageOverlay {
    fn dropdown_ui(&mut self, _ui: &mut Ui) {}
    fn reset_settings(&mut self) {}
    fn inputs(&self) -> Vec<OverlayInput>;
    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report>;
    fn image_overlay(&self, painter: &TwixPainter<Pixel>);
}

struct OverlayEntry {
    id: &'static str,
    name: &'static str,
    enabled: bool,
    overlay: Box<dyn ImageOverlay>,
    slots: Range<usize>,
    frame: Option<Time>,
    error: Option<String>,
    inputs: OverlayInputs,
}

impl OverlayEntry {
    fn new(id: &'static str, name: &'static str, overlay: impl ImageOverlay + 'static) -> Self {
        Self {
            id,
            name,
            enabled: false,
            overlay: Box::new(overlay),
            slots: 0..0,
            frame: None,
            error: None,
            inputs: OverlayInputs::default(),
        }
    }

    fn clear(&mut self) {
        self.frame = None;
        self.error = None;
        self.inputs.clear();
        let _ = self.overlay.prepare(Time::zero(), &mut self.inputs);
    }
}

pub(super) struct ImageOverlays {
    entries: Vec<OverlayEntry>,
    pub revision: u64,
}

impl ImageOverlays {
    pub(super) fn new(value: Option<&Value>) -> Self {
        let mut entries = vec![
            OverlayEntry::new("horizon", "Horizon", HorizonOverlay::default()),
            OverlayEntry::new(
                "object_detection",
                "Object Detection",
                ObjectDetectionOverlay::default(),
            ),
            OverlayEntry::new(
                "pose_detection",
                "Pose Detection",
                PoseDetectionOverlay::default(),
            ),
        ];
        for entry in &mut entries {
            entry.enabled = value
                .and_then(|v| v.get(entry.id))
                .and_then(|v| v.get("active"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
        }
        Self {
            entries,
            revision: 0,
        }
    }

    pub(super) fn declarations(
        &mut self,
        namespace: &str,
        supported: bool,
    ) -> Result<(Vec<InputSpec>, Vec<Source>), Report> {
        let mut inputs = Vec::new();
        let mut preferences = Vec::new();
        for entry in &mut self.entries {
            entry.clear();
            let start = inputs.len();
            if supported && entry.enabled {
                for input in entry.overlay.inputs() {
                    let prefer_frame = input.prefer_frame;
                    let spec = input.into_spec(namespace)?;
                    if prefer_frame {
                        preferences.push(spec.source.clone());
                    }
                    inputs.push(spec);
                }
            }
            entry.slots = start..inputs.len();
        }
        Ok((inputs, preferences))
    }

    pub(super) fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }

    pub(super) fn prepare(&mut self, frame: &Frame) {
        let time = frame.primary.record.time;
        for entry in &mut self.entries {
            if !entry.enabled || entry.slots.is_empty() {
                continue;
            }
            let Some(inputs) = frame.inputs.get(entry.slots.clone()) else {
                entry.clear();
                continue;
            };
            let changed = entry.inputs.update(inputs);
            if changed || entry.frame != Some(time) {
                entry.frame = Some(time);
                entry.inputs.begin_prepare();
                entry.error = entry
                    .overlay
                    .prepare(time, &mut entry.inputs)
                    .err()
                    .map(|e| format!("{e:#}"))
                    .or_else(|| entry.inputs.errors().next().map(str::to_owned));
            }
        }
    }

    pub(super) fn paint(&self, painter: &TwixPainter<Pixel>, time: Time) {
        for entry in &self.entries {
            if entry.enabled && entry.frame == Some(time) {
                entry.overlay.image_overlay(painter);
            }
        }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) {
        MenuButton::new("Overlays")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                for entry in &mut self.entries {
                    if ui.checkbox(&mut entry.enabled, entry.name).changed() {
                        entry.overlay.reset_settings();
                        self.revision = self.revision.wrapping_add(1);
                    }
                    if entry.enabled {
                        ui.indent(entry.id, |ui| entry.overlay.dropdown_ui(ui));
                    }
                    if let Some(error) = &entry.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }
            });
    }

    pub(super) fn save(&self) -> Value {
        Value::Object(
            self.entries
                .iter()
                .map(|entry| (entry.id.into(), json!({"active":entry.enabled})))
                .collect(),
        )
    }
}
