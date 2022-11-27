use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{
    configuration::{EdgeDetectionSource, MedianMode},
    CameraMatrix, FieldColor, ImageSegments, ProjectedLimbs,
};

pub struct ImageSegmenter {}

#[context]
pub struct NewContext {
    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "image_segmenter/$cycler_instance/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "image_segmenter/$cycler_instance/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "image_segmenter/$cycler_instance/vertical_median_mode">,
}

#[context]
pub struct CycleContext {
    pub image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    pub image: Input<Arc<bool>, "image">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub field_color: Input<FieldColor, "field_color">,
    pub projected_limbs: RequiredInput<Option<ProjectedLimbs>, "Control", "projected_limbs?">,

    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "image_segmenter/$cycler_instance/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "image_segmenter/$cycler_instance/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "image_segmenter/$cycler_instance/vertical_median_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<Option<ImageSegments>>,
}

impl ImageSegmenter {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
