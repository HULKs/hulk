use std::sync::Arc;

use color_eyre::Result;
use communication::client::Cycler;
use convert_case::Casing;
use eframe::egui::Ui;
use serde_json::{json, Value};

use crate::{nao::Nao, twix_painter::TwixPainter};

use super::overlays::{BallDetection, FeetDetection, LineDetection, PenaltyBoxes};

pub trait Overlay {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self;
    fn paint(&self, painter: &TwixPainter) -> Result<()>;
}

pub struct EnabledOverlay<T>
where
    T: Overlay,
{
    nao: Arc<Nao>,
    overlay: Option<T>,
    active: bool,
}

impl<T> EnabledOverlay<T>
where
    T: Overlay,
{
    pub fn new(
        nao: Arc<Nao>,
        value: Option<&Value>,
        active: bool,
        selected_cycler: Cycler,
    ) -> Self {
        let active = value
            .and_then(|value| value.get(T::NAME.to_case(convert_case::Case::Snake)))
            .and_then(|value| value.get("active"))
            .and_then(|value| value.as_bool())
            .unwrap_or(active);
        let layer = active.then(|| T::new(nao.clone(), selected_cycler));
        Self {
            nao,
            overlay: layer,
            active,
        }
    }

    pub fn update_cycler(&mut self, selected_cycler: Cycler) {
        if let Some(overlay) = self.overlay.as_mut() {
            *overlay = T::new(self.nao.clone(), selected_cycler);
        }
    }

    pub fn checkbox(&mut self, ui: &mut Ui, selected_cycler: Cycler) {
        if ui.checkbox(&mut self.active, T::NAME).changed() {
            match (self.active, self.overlay.is_some()) {
                (true, false) => self.overlay = Some(T::new(self.nao.clone(), selected_cycler)),
                (false, true) => self.overlay = None,
                _ => {}
            }
        }
    }

    pub fn paint(&self, painter: &TwixPainter) -> Result<()> {
        if let Some(layer) = &self.overlay {
            layer.paint(painter)?;
        }
        Ok(())
    }

    pub fn save(&self) -> Value {
        json!({"active": self.active})
    }
}

pub struct Overlays {
    pub line_detection: EnabledOverlay<LineDetection>,
    pub ball_detection: EnabledOverlay<BallDetection>,
    pub penalty_boxes: EnabledOverlay<PenaltyBoxes>,
    pub feet_detection: EnabledOverlay<FeetDetection>,
}

impl Overlays {
    pub fn new(nao: Arc<Nao>, storage: Option<&Value>, selected_cycler: Cycler) -> Self {
        let line_detection = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let ball_detection = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let penalty_boxes = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let feet_detection = EnabledOverlay::new(nao, storage, true, selected_cycler);
        Self {
            line_detection,
            ball_detection,
            penalty_boxes,
            feet_detection,
        }
    }

    pub fn update_cycler(&mut self, selected_cycler: Cycler) {
        self.line_detection.update_cycler(selected_cycler);
        self.ball_detection.update_cycler(selected_cycler);
        self.penalty_boxes.update_cycler(selected_cycler);
        self.feet_detection.update_cycler(selected_cycler);
    }

    pub fn combo_box(&mut self, ui: &mut Ui, selected_cycler: Cycler) {
        ui.menu_button("Overlays", |ui| {
            self.line_detection.checkbox(ui, selected_cycler);
            self.ball_detection.checkbox(ui, selected_cycler);
            self.penalty_boxes.checkbox(ui, selected_cycler);
            self.feet_detection.checkbox(ui, selected_cycler);
        });
    }

    pub fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let _ = self.line_detection.paint(painter);
        let _ = self.ball_detection.paint(painter);
        let _ = self.penalty_boxes.paint(painter);
        let _ = self.feet_detection.paint(painter);
        Ok(())
    }

    pub fn save(&self) -> Value {
        json!({
            "line_detection": self.line_detection.save(),
            "ball_detection": self.ball_detection.save(),
            "penalty_boxes": self.penalty_boxes.save(),
            "feet_detection": self.feet_detection.save(),
        })
    }
}
