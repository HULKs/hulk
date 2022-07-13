use std::sync::Arc;

use anyhow::Result;
use eframe::{egui::Ui, Storage};

use types::FieldDimensions;

use crate::{nao::Nao, twix_paint::TwixPainter};

pub trait Layer {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>) -> Self;
    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) -> Result<()>;
}

pub struct EnabledLayer<T>
where
    T: Layer,
{
    nao: Arc<Nao>,
    layer: Option<T>,
    active: bool,
}

impl<T> EnabledLayer<T>
where
    T: Layer,
{
    pub fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>, active: bool) -> Self {
        let active = storage
            .and_then(|storage| storage.get_string(&format!("map.{}", T::NAME)))
            .and_then(|value| value.parse().ok())
            .unwrap_or(active);
        let layer = active.then(|| T::new(nao.clone()));
        Self { nao, layer, active }
    }

    pub fn checkbox(&mut self, ui: &mut Ui) {
        if ui.checkbox(&mut self.active, T::NAME).changed() {
            match (self.active, self.layer.is_some()) {
                (true, false) => self.layer = Some(T::new(self.nao.clone())),
                (false, true) => self.layer = None,
                _ => {}
            }
        }
    }

    pub fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) -> Result<()> {
        if let Some(layer) = &self.layer {
            layer.paint(painter, field_dimensions)?;
        }
        Ok(())
    }

    pub fn save(&self, storage: &mut dyn Storage) {
        storage.set_string(&format!("map.{}", T::NAME), self.active.to_string());
    }
}
