use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Camera, Field};
use framework::MainOutput;
use linear_algebra::{point, Isometry3, Point3};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Deserialize, Serialize)]
pub struct FeatureProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    // field_to_camera: Input<Isometry3<Field, Camera>, "field_to_camera">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub feature: MainOutput<Option<Point3<Camera>>>,
}

impl FeatureProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let field_to_camera = Isometry3::from_translation(0.0, 0.0, 0.8);

        //TODO: implement faked feature extraction by using left upper corner of goal as point in field and return position in camera coordinate system
        let upper_left_goal_post: Point3<Field> = point![
            context.field_dimensions.length / 2.0,
            context.field_dimensions.goal_inner_width / 2.0
                + context.field_dimensions.goal_post_diameter / 2.0,
            context.field_dimensions.goal_inner_height,
        ];

        let upper_left_goal_post_in_camera = field_to_camera * upper_left_goal_post;

        //todo: look if point is actually visible in camera
        // compare that upper_left_goal_post_in_camera vector is between camera angles
        let field_of_view_width_angle = 60.0_f32.to_radians();
        let field_of_view_height_angle = 45.0_f32.to_radians();

        let z = upper_left_goal_post_in_camera.z();

        let horizontal_angle_to_object = upper_left_goal_post_in_camera.y().atan2(z);
        let vertical_angle_to_object = upper_left_goal_post_in_camera.x().atan2(z);

        // check if point is within field of view
        // point in front of camera
        let is_visible = z > 0.0
            && horizontal_angle_to_object.abs() < field_of_view_width_angle / 2.0
            && vertical_angle_to_object.abs() < field_of_view_height_angle / 2.0;

        let feature = if is_visible {
            Some(upper_left_goal_post_in_camera)
        } else {
            None
        };

        Ok(MainOutputs {
            feature: feature.into(),
        })
    }
}
