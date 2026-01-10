use std::f32::consts::{FRAC_PI_2, PI};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Camera, Field, World};
use framework::MainOutput;
use linear_algebra::{point, vector, Isometry3, Orientation3, Point3};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Deserialize, Serialize)]
pub struct FeatureProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_to_world: Input<Isometry3<Camera, World>, "camera_to_world">,
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
        // let field_to_camera = Isometry3::from_parts(
        //     vector![-0.75, 0.0, 0.0],
        //     Orientation3::from_euler_angles(PI, -FRAC_PI_2, 0.0),
        // );
        let world_to_camera = context.camera_to_world.inverse();

        let upper_left_goal_post = point![
            context.field_dimensions.length / 2.0,
            context.field_dimensions.goal_inner_width / 2.0
                + context.field_dimensions.goal_post_diameter / 2.0,
            context.field_dimensions.goal_inner_height,
        ];

        let upper_left_goal_post_in_camera = world_to_camera * upper_left_goal_post;

        let field_of_view_width_angle = 60.0_f32.to_radians();
        let field_of_view_height_angle = 45.0_f32.to_radians();

        let x = upper_left_goal_post_in_camera.z();

        TODO
        let horizontal_angle_to_object = upper_left_goal_post_in_camera.y().atan2(x);
        let vertical_angle_to_object = upper_left_goal_post_in_camera.z().atan2(x);

        let is_visible = x > 0.0
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
