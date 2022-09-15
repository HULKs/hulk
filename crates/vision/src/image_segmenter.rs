use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput, Parameter};

pub struct ImageSegmenter {}

#[context]
pub struct NewContext {
    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "$this_cycler/image_segmenter/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "$this_cycler/image_segmenter/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "$this_cycler/image_segmenter/vertical_median_mode">,
}

#[context]
pub struct CycleContext {
    pub image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    pub camera_matrix: OptionalInput<CameraMatrix, "camera_matrix">,
    pub field_color: OptionalInput<FieldColor, "field_color">,
    pub projected_limbs: OptionalInput<ProjectedLimbs, "Control", "projected_limbs">,

    pub vertical_edge_detection_source: Parameter<
        EdgeDetectionSource,
        "$this_cycler/image_segmenter/vertical_edge_detection_source",
    >,
    pub vertical_edge_threshold:
        Parameter<i16, "$this_cycler/image_segmenter/vertical_edge_threshold">,
    pub vertical_median_mode:
        Parameter<MedianMode, "$this_cycler/image_segmenter/vertical_median_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<ImageSegments>,
}

impl ImageSegmenter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
