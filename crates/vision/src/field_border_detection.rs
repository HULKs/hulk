use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};

pub struct FieldBorderDetection {}

#[context]
pub struct NewContext {
    pub angle_threshold: Parameter<f32, "$cycler_instance/field_border_detection/angle_threshold">,
    pub first_line_association_distance:
        Parameter<f32, "$cycler_instance/field_border_detection/first_line_association_distance">,
    pub horizon_margin: Parameter<f32, "$cycler_instance/field_border_detection/horizon_margin">,
    pub min_points_per_line:
        Parameter<usize, "$cycler_instance/field_border_detection/min_points_per_line">,
    pub second_line_association_distance:
        Parameter<f32, "$cycler_instance/field_border_detection/second_line_association_distance">,
}

#[context]
pub struct CycleContext {
    pub field_border_points: AdditionalOutput<Vec<Point2<f32>>, "field_border_points">,

    pub angle_threshold: Parameter<f32, "$cycler_instance/field_border_detection/angle_threshold">,
    pub first_line_association_distance:
        Parameter<f32, "$cycler_instance/field_border_detection/first_line_association_distance">,
    pub horizon_margin: Parameter<f32, "$cycler_instance/field_border_detection/horizon_margin">,
    pub min_points_per_line:
        Parameter<usize, "$cycler_instance/field_border_detection/min_points_per_line">,
    pub second_line_association_distance:
        Parameter<f32, "$cycler_instance/field_border_detection/second_line_association_distance">,

    pub camera_matrix: RequiredInput<CameraMatrix, "camera_matrix">,
    pub image_segments: RequiredInput<ImageSegments, "image_segments">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_border: MainOutput<FieldBorder>,
}

impl FieldBorderDetection {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
