use std::{marker::PhantomData, sync::Arc};

use color_eyre::Result;
use convert_case::{Case, Casing};
use eframe::egui::Ui;
use serde_json::{json, Value};

use types::field_dimensions::FieldDimensions;

use crate::{nao::Nao, twix_painter::TwixPainter};

pub trait Layer<Frame> {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>) -> Self;
    fn paint(&self, painter: &TwixPainter<Frame>, field_dimensions: &FieldDimensions)
        -> Result<()>;
}

pub struct EnabledLayer<T, Frame>
where
    T: Layer<Frame>,
{
    nao: Arc<Nao>,
    layer: Option<T>,
    active: bool,
    frame: PhantomData<Frame>,
}

impl<T, Frame> EnabledLayer<T, Frame>
where
    T: Layer<Frame>,
{
    pub fn new(nao: Arc<Nao>, value: Option<&Value>, active: bool) -> Self {
        let active = value
            .and_then(|value| value.get(T::NAME.to_case(Case::Snake)))
            .and_then(|value| value.get("active"))
            .and_then(|value| value.as_bool())
            .unwrap_or(active);
        let layer = active.then(|| T::new(nao.clone()));
        Self {
            nao,
            layer,
            active,
            frame: PhantomData,
        }
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

    pub fn paint(
        &self,
        painter: &TwixPainter<Frame>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(layer) = &self.layer {
            layer.paint(painter, field_dimensions)?;
        }
        Ok(())
    }

    pub fn save(&self) -> Value {
        json!({
            "active": self.active
        })
    }
}
