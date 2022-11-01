use std::ops::Range;

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput, Parameter};
use types::{CameraMatrix, FilteredSegments, ImageLines, LineData};

pub struct LineDetection {}

#[context]
pub struct NewContext {
    pub allowed_line_length_in_field:
        Parameter<Range<f32>, "$cycler_instance/line_detection/allowed_line_length_in_field">,
    pub check_line_distance: Parameter<bool, "$cycler_instance/line_detection/check_line_distance">,
    pub check_line_length: Parameter<bool, "$cycler_instance/line_detection/check_line_length">,
    pub check_line_segments_projection:
        Parameter<bool, "$cycler_instance/line_detection/check_line_segments_projection">,
    pub gradient_alignment: Parameter<f32, "$cycler_instance/line_detection/gradient_alignment">,
    pub maximum_distance_to_robot:
        Parameter<f32, "$cycler_instance/line_detection/maximum_distance_to_robot">,
    pub maximum_fit_distance_in_pixels:
        Parameter<f32, "$cycler_instance/line_detection/maximum_fit_distance_in_pixels">,
    pub maximum_gap_on_line: Parameter<f32, "$cycler_instance/line_detection/maximum_gap_on_line">,
    pub maximum_number_of_lines:
        Parameter<usize, "$cycler_instance/line_detection/maximum_number_of_lines">,
    pub maximum_projected_segment_length:
        Parameter<f32, "$cycler_instance/line_detection/maximum_projected_segment_length">,
    pub minimum_number_of_points_on_line:
        Parameter<usize, "$cycler_instance/line_detection/minimum_number_of_points_on_line">,
}

#[context]
pub struct CycleContext {
    pub lines_in_image: AdditionalOutput<ImageLines, "lines_in_image">,

    pub camera_matrix: OptionalInput<CameraMatrix, "camera_matrix?">,
    pub filtered_segments: OptionalInput<FilteredSegments, "filtered_segments?">,

    pub allowed_line_length_in_field:
        Parameter<Range<f32>, "$cycler_instance/line_detection/allowed_line_length_in_field">,
    pub check_line_distance: Parameter<bool, "$cycler_instance/line_detection/check_line_distance">,
    pub check_line_length: Parameter<bool, "$cycler_instance/line_detection/check_line_length">,
    pub check_line_segments_projection:
        Parameter<bool, "$cycler_instance/line_detection/check_line_segments_projection">,
    pub gradient_alignment: Parameter<f32, "$cycler_instance/line_detection/gradient_alignment">,
    pub maximum_distance_to_robot:
        Parameter<f32, "$cycler_instance/line_detection/maximum_distance_to_robot">,
    pub maximum_fit_distance_in_pixels:
        Parameter<f32, "$cycler_instance/line_detection/maximum_fit_distance_in_pixels">,
    pub maximum_gap_on_line: Parameter<f32, "$cycler_instance/line_detection/maximum_gap_on_line">,
    pub maximum_number_of_lines:
        Parameter<usize, "$cycler_instance/line_detection/maximum_number_of_lines">,
    pub maximum_projected_segment_length:
        Parameter<f32, "$cycler_instance/line_detection/maximum_projected_segment_length">,
    pub minimum_number_of_points_on_line:
        Parameter<usize, "$cycler_instance/line_detection/minimum_number_of_points_on_line">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub line_data: MainOutput<LineData>,
}

impl LineDetection {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
