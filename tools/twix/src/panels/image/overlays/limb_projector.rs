use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use types::limb::ProjectedLimbs;

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct LimbProjector {
    projected_limbs: BufferHandle<Option<ProjectedLimbs>>,
}

impl Overlay for LimbProjector {
    const NAME: &'static str = "Projected Limbs";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            projected_limbs: nao
                .subscribe_value(format!("{cycler_path}.main_outputs.projected_limbs")),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(projected_limbs) = self.projected_limbs.get_last_value()?.flatten() else {
            return Ok(());
        };
        for limb in projected_limbs.limbs {
            painter.polygon(limb.pixel_polygon, Stroke::new(3.0, Color32::WHITE));
        }
        Ok(())
    }
}
