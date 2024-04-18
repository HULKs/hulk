use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use types::limb::ProjectedLimbs;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LimbProjector {
    projected_limbs: ValueBuffer,
}

impl Overlay for LimbProjector {
    const NAME: &'static str = "Projected Limbs";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            projected_limbs: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{selected_cycler}.main_outputs.projected_limbs"))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let projected_limbs: ProjectedLimbs = self.projected_limbs.require_latest()?;
        for limb in projected_limbs.limbs {
            painter.polygon(limb.pixel_polygon, Stroke::new(3.0, Color32::WHITE));
        }
        Ok(())
    }
}
