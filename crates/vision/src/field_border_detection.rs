use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::Point2;
use types::{CameraMatrix, FieldBorder, ImageSegments};

pub struct FieldBorderDetection {}

#[context]
pub struct CreationContext {
    pub angle_threshold: Parameter<f32, "field_border_detection/$cycler_instance/angle_threshold">,
    pub first_line_association_distance:
        Parameter<f32, "field_border_detection/$cycler_instance/first_line_association_distance">,
    pub horizon_margin: Parameter<f32, "field_border_detection/$cycler_instance/horizon_margin">,
    pub min_points_per_line:
        Parameter<usize, "field_border_detection/$cycler_instance/min_points_per_line">,
    pub second_line_association_distance:
        Parameter<f32, "field_border_detection/$cycler_instance/second_line_association_distance">,
}

#[context]
pub struct CycleContext {
    pub field_border_points: AdditionalOutput<Vec<Point2<f32>>, "field_border_points">,

    pub angle_threshold: Parameter<f32, "field_border_detection/$cycler_instance/angle_threshold">,
    pub first_line_association_distance:
        Parameter<f32, "field_border_detection/$cycler_instance/first_line_association_distance">,
    pub horizon_margin: Parameter<f32, "field_border_detection/$cycler_instance/horizon_margin">,
    pub min_points_per_line:
        Parameter<usize, "field_border_detection/$cycler_instance/min_points_per_line">,
    pub second_line_association_distance:
        Parameter<f32, "field_border_detection/$cycler_instance/second_line_association_distance">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub image_segments: RequiredInput<Option<ImageSegments>, "image_segments?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_border: MainOutput<Option<FieldBorder>>,
}

impl FieldBorderDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
