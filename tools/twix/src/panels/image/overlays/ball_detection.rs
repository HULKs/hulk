use color_eyre::{Report, eyre::bail};
use coordinate_systems::Pixel;
use ros_z::time::Time;

use super::super::image_overlay::ImageOverlay;
use crate::repaint::ObservationContext;
use twix_visualization::twix_painter::TwixPainter;

pub(in crate::panels::image) struct BallDetectionOverlay;

impl ImageOverlay for BallDetectionOverlay {
    type Sample = ();
    const NAME: &'static str = "Ball Detection";
    const STORAGE_KEY: &'static str = "ball_detection";

    fn new<C: ObservationContext>(_: &C) -> Result<Self, Report> {
        bail!("omitted: filtered ball pixels have no image timestamp")
    }
    fn prepare(&self, _: Time) -> Option<()> {
        None
    }
    fn paint(_: &TwixPainter<Pixel>, _: &()) {}
}
