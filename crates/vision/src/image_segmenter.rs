use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{hardware::Image, CameraMatrix, FieldColor, ImageSegments, ProjectedLimbs};

pub struct ImageSegmenter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    pub image: Input<Image, "image">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub field_color: Input<FieldColor, "field_color">,
    pub projected_limbs: RequiredInput<Option<ProjectedLimbs>, "Control", "projected_limbs?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<Option<ImageSegments>>,
}

impl ImageSegmenter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
