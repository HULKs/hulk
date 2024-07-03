use std::sync::Arc;

use color_eyre::Result;
use communication::client::Cycler;
use convert_case::Casing;
use coordinate_systems::Pixel;
use eframe::egui::Ui;
use serde_json::{json, Value};

use crate::{nao::Nao, twix_painter::TwixPainter};

use super::overlays::{
    BallDetection, FeetDetection, FieldBorder, Horizon, LimbProjector, LineDetection, PenaltyBoxes,
    PerspectiveGrid, PoseDetection,
};
pub trait Overlay {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self;
    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()>;
    fn config_ui(&mut self, _ui: &mut Ui) {}
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
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.config_ui(ui);
        }
    }

    pub fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
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
    pub perspective_grid: EnabledOverlay<PerspectiveGrid>,
    pub horizon: EnabledOverlay<Horizon>,
    pub penalty_boxes: EnabledOverlay<PenaltyBoxes>,
    pub feet_detection: EnabledOverlay<FeetDetection>,
    pub field_border: EnabledOverlay<FieldBorder>,
    pub limb_projector: EnabledOverlay<LimbProjector>,
    pub pose_detection: EnabledOverlay<PoseDetection>,
}

impl Overlays {
    pub fn new(nao: Arc<Nao>, storage: Option<&Value>, selected_cycler: Cycler) -> Self {
        let line_detection = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let ball_detection = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let perspective_grid = EnabledOverlay::new(nao.clone(), storage, false, selected_cycler);
        let horizon = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let penalty_boxes = EnabledOverlay::new(nao.clone(), storage, false, selected_cycler);
        let feet_detection = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let field_border = EnabledOverlay::new(nao.clone(), storage, true, selected_cycler);
        let limb_projector = EnabledOverlay::new(nao.clone(), storage, false, selected_cycler);
        let pose_detection = EnabledOverlay::new(nao, storage, true, selected_cycler);

        Self {
            line_detection,
            ball_detection,
            perspective_grid,
            horizon,
            penalty_boxes,
            feet_detection,
            field_border,
            limb_projector,
            pose_detection,
        }
    }

    pub fn update_cycler(&mut self, selected_cycler: Cycler) {
        self.line_detection.update_cycler(selected_cycler);
        self.ball_detection.update_cycler(selected_cycler);
        self.perspective_grid.update_cycler(selected_cycler);
        self.horizon.update_cycler(selected_cycler);
        self.penalty_boxes.update_cycler(selected_cycler);
        self.feet_detection.update_cycler(selected_cycler);
        self.field_border.update_cycler(selected_cycler);
        self.limb_projector.update_cycler(selected_cycler);
        self.pose_detection.update_cycler(selected_cycler);
    }

    pub fn combo_box(&mut self, ui: &mut Ui, selected_cycler: Cycler) {
        ui.menu_button("Overlays", |ui| {
            self.line_detection.checkbox(ui, selected_cycler);
            self.ball_detection.checkbox(ui, selected_cycler);
            self.perspective_grid.checkbox(ui, selected_cycler);
            self.horizon.checkbox(ui, selected_cycler);
            self.penalty_boxes.checkbox(ui, selected_cycler);
            self.feet_detection.checkbox(ui, selected_cycler);
            self.field_border.checkbox(ui, selected_cycler);
            self.limb_projector.checkbox(ui, selected_cycler);
            self.pose_detection.checkbox(ui, selected_cycler);
        });
    }

    pub fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let _ = self.line_detection.paint(painter);
        let _ = self.ball_detection.paint(painter);
        let _ = self.perspective_grid.paint(painter);
        let _ = self.horizon.paint(painter);
        let _ = self.penalty_boxes.paint(painter);
        let _ = self.feet_detection.paint(painter);
        let _ = self.field_border.paint(painter);
        let _ = self.limb_projector.paint(painter);
        let _ = self.pose_detection.paint(painter);
        Ok(())
    }

    pub fn save(&self) -> Value {
        json!({
            "line_detection": self.line_detection.save(),
            "ball_detection": self.ball_detection.save(),
            "perspective_grid": self.perspective_grid.save(),
            "horizon": self.horizon.save(),
            "penalty_boxes": self.penalty_boxes.save(),
            "feet_detection": self.feet_detection.save(),
            "field_border": self.field_border.save(),
            "limb_projector": self.limb_projector.save(),
            "pose_detection": self.pose_detection.save(),
        })
    }
}
