use std::sync::Arc;

use color_eyre::Result;
use convert_case::Casing;
use eframe::egui::Ui;
use log::error;
use serde_json::{Value, json};

use coordinate_systems::Pixel;

use crate::{
    panels::image::overlays::{BallDetection, ObjectDetection, Segmentation},
    robot::Robot,
    twix_painter::TwixPainter,
};

use super::overlays::{FieldBorder, Horizon, LineDetection};

pub trait Overlay {
    const NAME: &'static str;
    fn new(robot: Arc<Robot>) -> Self;
    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()>;
    fn config_ui(&mut self, _ui: &mut Ui) {}
}

pub struct EnabledOverlay<T>
where
    T: Overlay,
{
    robot: Arc<Robot>,
    overlay: Option<T>,
}

impl<T> EnabledOverlay<T>
where
    T: Overlay,
{
    pub fn new(robot: Arc<Robot>, value: Option<&Value>, active: bool) -> Self {
        let active = value
            .and_then(|value| value.get(T::NAME.to_case(convert_case::Case::Snake)))
            .and_then(|value| value.get("active"))
            .and_then(|value| value.as_bool())
            .unwrap_or(active);
        let overlay = active.then(|| T::new(robot.clone()));
        Self { robot, overlay }
    }

    pub fn checkbox(&mut self, ui: &mut Ui) {
        let mut active = self.overlay.is_some();
        if ui.checkbox(&mut active, T::NAME).changed() {
            match self.overlay.is_some() {
                false => self.overlay = Some(T::new(self.robot.clone())),
                true => self.overlay = None,
            }
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.config_ui(ui);
        }
    }

    pub fn paint(&mut self, painter: &TwixPainter<Pixel>) {
        if let Some(layer) = &self.overlay
            && let Err(error) = layer.paint(painter)
        {
            error!("image panel: paint image overlay {}: {:#}", T::NAME, error);
            self.overlay = None;
        }
    }

    pub fn save(&self) -> Value {
        json!({"active": self.overlay.is_some()})
    }
}

pub struct Overlays {
    pub line_detection: EnabledOverlay<LineDetection>,
    pub ball_detection: EnabledOverlay<BallDetection>,
    pub horizon: EnabledOverlay<Horizon>,
    pub field_border: EnabledOverlay<FieldBorder>,
    pub object_detection: EnabledOverlay<ObjectDetection>,
    pub segmentation: EnabledOverlay<Segmentation>,
}

impl Overlays {
    pub fn new(robot: Arc<Robot>, storage: Option<&Value>) -> Self {
        let line_detection = EnabledOverlay::new(robot.clone(), storage, false);
        let ball_detection = EnabledOverlay::new(robot.clone(), storage, false);
        let horizon = EnabledOverlay::new(robot.clone(), storage, false);
        let field_border = EnabledOverlay::new(robot.clone(), storage, false);
        let object_detection = EnabledOverlay::new(robot.clone(), storage, false);
        let segmentation = EnabledOverlay::new(robot.clone(), storage, false);

        Self {
            line_detection,
            ball_detection,
            horizon,
            field_border,
            object_detection,
            segmentation,
        }
    }

    pub fn combo_box(&mut self, ui: &mut Ui) {
        ui.menu_button("Overlays", |ui| {
            self.line_detection.checkbox(ui);
            self.ball_detection.checkbox(ui);
            self.horizon.checkbox(ui);
            self.field_border.checkbox(ui);
            self.object_detection.checkbox(ui);
            self.segmentation.checkbox(ui);
        });
    }

    pub fn paint(&mut self, painter: &TwixPainter<Pixel>) {
        self.line_detection.paint(painter);
        self.ball_detection.paint(painter);
        self.horizon.paint(painter);
        self.field_border.paint(painter);
        self.object_detection.paint(painter);
        self.segmentation.paint(painter);
    }

    pub fn save(&self) -> Value {
        json!({
            "line_detection": self.line_detection.save(),
            "ball_detection": self.ball_detection.save(),
            "horizon": self.horizon.save(),
            "field_border": self.field_border.save(),
            "object_detection": self.object_detection.save(),
            "segmentation": self.object_detection.save(),
        })
    }
}
