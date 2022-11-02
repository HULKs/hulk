use std::{sync::Arc, time::Duration};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput, Parameter, RequiredInput};
use types::{
    configuration::{EdgeDetectionSource, MedianMode},
    CameraMatrix, FieldColor, Image422, ImageSegments, ProjectedLimbs,
};

pub struct ImageSegmenter {}

#[context]
pub struct NewContext {
    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "$cycler_instance/image_segmenter/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "$cycler_instance/image_segmenter/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "$cycler_instance/image_segmenter/vertical_median_mode">,
}

#[context]
pub struct CycleContext {
    pub image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    pub image: RequiredInput<Arc<Image422>, "image">,

    pub camera_matrix: OptionalInput<CameraMatrix, "camera_matrix?">,
    pub field_color: OptionalInput<FieldColor, "field_color?">,
    pub projected_limbs: OptionalInput<ProjectedLimbs, "Control", "projected_limbs?">,

    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "$cycler_instance/image_segmenter/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "$cycler_instance/image_segmenter/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "$cycler_instance/image_segmenter/vertical_median_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<ImageSegments>,
}

impl ImageSegmenter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
