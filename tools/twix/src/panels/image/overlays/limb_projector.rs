use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use types::limb::ProjectedLimbs;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct LimbProjector {
    projected_limbs: BufferHandle<Option<ProjectedLimbs>>,
}

impl Overlay for LimbProjector {
    const NAME: &'static str = "Projected Limbs";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        Self {
            projected_limbs: nao.subscribe_value("Vision.main_outputs.projected_limbs"),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(projected_limbs) = self.projected_limbs.get_last_value()?.flatten() else {
            return Ok(());
        };
        for limb in projected_limbs.limbs {
            painter.polyline(limb.pixel_polyline, Stroke::new(3.0, Color32::WHITE));
        }
        Ok(())
    }
}
