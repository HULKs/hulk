use std::{marker::PhantomData, sync::Arc};

use color_eyre::Result;
use convert_case::{Case, Casing};
use eframe::egui::Ui;
use log::error;
use serde_json::{Value, json};

use types::field_dimensions::FieldDimensions;

use crate::{
    backend::RobotBackend,
    panel::PanelUiContext,
    repaint::{ObservationContext, ObservationRepaint},
};
use twix_visualization::twix_painter::TwixPainter;

pub trait Layer<Frame> {
    const NAME: &'static str;
    // Explicit keys distinguish controls whose names or semantics changed.
    const STORAGE_KEY: Option<&'static str> = None;
    fn new(backend: Arc<RobotBackend>) -> Self;
    fn paint(&self, painter: &TwixPainter<Frame>, field_dimensions: &FieldDimensions)
    -> Result<()>;

    fn repaint_on_updates(&self, _context: &impl ObservationContext) -> Vec<ObservationRepaint> {
        vec![]
    }

    fn status(&self) -> Option<String> {
        None
    }
}

pub struct EnabledLayer<T, Frame>
where
    T: Layer<Frame>,
{
    backend: Arc<RobotBackend>,
    layer: Option<T>,
    repaints: Vec<ObservationRepaint>,
    frame: PhantomData<Frame>,
}

impl<T, Frame> EnabledLayer<T, Frame>
where
    T: Layer<Frame>,
{
    pub fn new(context: &impl ObservationContext, value: Option<&Value>, active: bool) -> Self {
        let backend = context.backend().clone();
        let storage_key = T::STORAGE_KEY
            .map(str::to_owned)
            .unwrap_or_else(|| T::NAME.to_case(Case::Snake));
        let active = value
            .and_then(|value| value.get(storage_key))
            .and_then(|value| value.get("active"))
            .and_then(|value| value.as_bool())
            .unwrap_or(value.is_none() && active);
        let layer = active.then(|| T::new(backend.clone()));
        let repaints = layer
            .as_ref()
            .map(|layer| layer.repaint_on_updates(context))
            .unwrap_or_default();
        Self {
            repaints,
            backend,
            layer,
            frame: PhantomData,
        }
    }

    pub fn checkbox(&mut self, ui: &mut Ui) {
        let mut active = self.layer.is_some();
        if ui.checkbox(&mut active, T::NAME).changed() {
            match self.layer.is_some() {
                false => {
                    let layer = T::new(self.backend.clone());
                    self.repaints = layer.repaint_on_updates(&PanelUiContext {
                        backend: &self.backend,
                        egui_context: ui.ctx(),
                    });
                    self.layer = Some(layer);
                }
                true => {
                    self.layer = None;
                    self.repaints.clear();
                }
            }
        }
        if let Some(status) = self.layer.as_ref().and_then(Layer::status) {
            ui.small(status);
        }
    }

    pub fn paint_or_disable(
        &mut self,
        painter: &TwixPainter<Frame>,
        field_dimensions: &FieldDimensions,
    ) {
        if let Some(layer) = &self.layer
            && let Err(error) = layer.paint(painter, field_dimensions)
        {
            error!(
                "map panel: failed to paint map overlay {}: {:#}",
                T::NAME,
                error
            );
            self.layer = None;
            self.repaints.clear();
        }
    }

    pub fn save(&self) -> Value {
        json!({
            "active": self.layer.is_some(),
        })
    }

    pub fn is_active(&self) -> bool {
        self.layer.is_some()
    }

    pub fn status(&self) -> Option<String> {
        self.layer.as_ref().and_then(Layer::status)
    }
}
