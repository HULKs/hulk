use std::ops::Range;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{CameraMatrix, FilteredSegments, ImageLines, LineData};

pub struct LineDetection {}

#[context]
pub struct CreationContext {
    pub allowed_line_length_in_field:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_line_length_in_field">,
    pub check_line_distance: Parameter<bool, "line_detection.$cycler_instance.check_line_distance">,
    pub check_line_length: Parameter<bool, "line_detection.$cycler_instance.check_line_length">,
    pub check_line_segments_projection:
        Parameter<bool, "line_detection.$cycler_instance.check_line_segments_projection">,
    pub gradient_alignment: Parameter<f32, "line_detection.$cycler_instance.gradient_alignment">,
    pub maximum_distance_to_robot:
        Parameter<f32, "line_detection.$cycler_instance.maximum_distance_to_robot">,
    pub maximum_fit_distance_in_pixels:
        Parameter<f32, "line_detection.$cycler_instance.maximum_fit_distance_in_pixels">,
    pub maximum_gap_on_line: Parameter<f32, "line_detection.$cycler_instance.maximum_gap_on_line">,
    pub maximum_number_of_lines:
        Parameter<usize, "line_detection.$cycler_instance.maximum_number_of_lines">,
    pub maximum_projected_segment_length:
        Parameter<f32, "line_detection.$cycler_instance.maximum_projected_segment_length">,
    pub minimum_number_of_points_on_line:
        Parameter<usize, "line_detection.$cycler_instance.minimum_number_of_points_on_line">,
}

#[context]
pub struct CycleContext {
    pub lines_in_image: AdditionalOutput<ImageLines, "lines_in_image">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub filtered_segments: RequiredInput<Option<FilteredSegments>, "filtered_segments?">,

    pub allowed_line_length_in_field:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_line_length_in_field">,
    pub check_line_distance: Parameter<bool, "line_detection.$cycler_instance.check_line_distance">,
    pub check_line_length: Parameter<bool, "line_detection.$cycler_instance.check_line_length">,
    pub check_line_segments_projection:
        Parameter<bool, "line_detection.$cycler_instance.check_line_segments_projection">,
    pub gradient_alignment: Parameter<f32, "line_detection.$cycler_instance.gradient_alignment">,
    pub maximum_distance_to_robot:
        Parameter<f32, "line_detection.$cycler_instance.maximum_distance_to_robot">,
    pub maximum_fit_distance_in_pixels:
        Parameter<f32, "line_detection.$cycler_instance.maximum_fit_distance_in_pixels">,
    pub maximum_gap_on_line: Parameter<f32, "line_detection.$cycler_instance.maximum_gap_on_line">,
    pub maximum_number_of_lines:
        Parameter<usize, "line_detection.$cycler_instance.maximum_number_of_lines">,
    pub maximum_projected_segment_length:
        Parameter<f32, "line_detection.$cycler_instance.maximum_projected_segment_length">,
    pub minimum_number_of_points_on_line:
        Parameter<usize, "line_detection.$cycler_instance.minimum_number_of_points_on_line">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub line_data: MainOutput<Option<LineData>>,
}

impl LineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
