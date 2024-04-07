use std::f32::consts::FRAC_PI_2;

use color_eyre::Result;
use nalgebra::UnitQuaternion;
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Ground, Head, Pixel, Robot};
use framework::{AdditionalOutput, MainOutput};
use geometry::line::{Line, Line2};
use linear_algebra::{point, vector, IntoTransform, Isometry3, Vector3};
use types::{
    camera_matrix::ProjectedFieldLines, field_dimensions::FieldDimensions,
    parameters::CameraMatrixParameters, robot_dimensions::RobotDimensions,
    robot_kinematics::RobotKinematics,
};

#[derive(Deserialize, Serialize)]
pub struct CameraMatrixCalculator {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    projected_field_lines: AdditionalOutput<ProjectedFieldLines, "projected_field_lines">,

    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,

    bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_bottom">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_top">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrices: MainOutput<Option<CameraMatrices>>,
}

impl CameraMatrixCalculator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let image_size = vector![640.0, 480.0];
        let head_to_top_camera = head_to_camera(
            context.top_camera_matrix_parameters.extrinsic_rotations,
            context
                .top_camera_matrix_parameters
                .camera_pitch
                .to_radians(),
            RobotDimensions::HEAD_TO_TOP_CAMERA,
        );
        let top_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.top_camera_matrix_parameters.focal_lengths,
            context.top_camera_matrix_parameters.cc_optical_center,
            image_size,
            context.robot_to_ground.inverse(),
            context.robot_kinematics.head_to_robot.inverse(),
            head_to_top_camera,
        );

        let head_to_bottom_camera = head_to_camera(
            context.bottom_camera_matrix_parameters.extrinsic_rotations,
            context
                .bottom_camera_matrix_parameters
                .camera_pitch
                .to_radians(),
            RobotDimensions::HEAD_TO_TOP_CAMERA,
        );
        let bottom_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.bottom_camera_matrix_parameters.focal_lengths,
            context.bottom_camera_matrix_parameters.cc_optical_center,
            image_size,
            context.robot_to_ground.inverse(),
            context.robot_kinematics.head_to_robot.inverse(),
            head_to_bottom_camera,
        );

        let field_dimensions = context.field_dimensions;
        context
            .projected_field_lines
            .fill_if_subscribed(|| ProjectedFieldLines {
                top: project_penalty_area_on_images(field_dimensions, &top_camera_matrix)
                    .unwrap_or_default(),
                bottom: project_penalty_area_on_images(field_dimensions, &bottom_camera_matrix)
                    .unwrap_or_default(),
            });
        Ok(MainOutputs {
            camera_matrices: Some(CameraMatrices {
                top: top_camera_matrix,
                bottom: bottom_camera_matrix,
            })
            .into(),
        })
    }
}

fn project_penalty_area_on_images(
    field_dimensions: &FieldDimensions,
    camera_matrix: &CameraMatrix,
) -> Option<Vec<Line2<Pixel>>> {
    let field_length = &field_dimensions.length;
    let field_width = &field_dimensions.width;
    let penalty_area_length = &field_dimensions.penalty_area_length;
    let penalty_area_width = &field_dimensions.penalty_area_width;

    let penalty_top_left = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, penalty_area_width / 2.0])
        .ok()?;
    let penalty_top_right = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, -penalty_area_width / 2.0])
        .ok()?;
    let penalty_bottom_left = camera_matrix
        .ground_to_pixel(point![
            field_length / 2.0 - penalty_area_length,
            penalty_area_width / 2.0
        ])
        .ok()?;
    let penalty_bottom_right = camera_matrix
        .ground_to_pixel(point![
            field_length / 2.0 - penalty_area_length,
            -penalty_area_width / 2.0
        ])
        .ok()?;
    let corner_left = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, field_width / 2.0])
        .ok()?;
    let corner_right = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, -field_width / 2.0])
        .ok()?;

    Some(vec![
        Line(penalty_top_left, penalty_top_right),
        Line(penalty_bottom_left, penalty_bottom_right),
        Line(penalty_bottom_left, penalty_top_left),
        Line(penalty_bottom_right, penalty_top_right),
        Line(corner_left, corner_right),
    ])
}

fn head_to_camera(
    extrinsic_rotation: nalgebra::Vector3<f32>,
    camera_pitch: f32,
    head_to_camera: Vector3<Head>,
) -> Isometry3<Head, Camera> {
    let extrinsic_angles_in_radians = extrinsic_rotation.map(|degree: f32| degree.to_radians());
    let extrinsic_rotation = UnitQuaternion::from_euler_angles(
        extrinsic_angles_in_radians.x,
        extrinsic_angles_in_radians.y,
        extrinsic_angles_in_radians.z,
    );

    (extrinsic_rotation
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * -camera_pitch)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * -FRAC_PI_2)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * FRAC_PI_2)
        * nalgebra::Isometry3::from(-head_to_camera.inner))
    .framed_transform()
}
