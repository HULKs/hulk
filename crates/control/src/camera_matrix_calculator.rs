use std::f32::consts::FRAC_PI_2;

use color_eyre::Result;
use nalgebra::UnitQuaternion;
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Field, Ground, Head, Pixel, Robot};
use framework::{AdditionalOutput, MainOutput};
use geometry::line_segment::LineSegment;
use linear_algebra::{point, vector, IntoTransform, Isometry2, Isometry3, Rotation3, Vector3};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    field_lines::ProjectedFieldLines,
    parameters::CameraMatrixParameters,
    robot_dimensions::RobotDimensions,
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
    ground_to_field: CyclerState<Isometry2<Ground, Field>, "ground_to_field">,

    bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_bottom">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_top">,

    correction_in_robot: Parameter<
        nalgebra::Vector3<f32>,
        "camera_matrix_parameters.calibration.correction_in_robot",
    >,
    correction_in_camera_top: Parameter<
        nalgebra::Vector3<f32>,
        "camera_matrix_parameters.calibration.correction_in_camera_top",
    >,
    correction_in_camera_bottom: Parameter<
        nalgebra::Vector3<f32>,
        "camera_matrix_parameters.calibration.correction_in_camera_bottom",
    >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub uncalibrated_camera_matrices: MainOutput<Option<CameraMatrices>>,
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
        let uncalibrated_top_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.top_camera_matrix_parameters.focal_lengths,
            context.top_camera_matrix_parameters.cc_optical_center,
            image_size,
            context.robot_to_ground.inverse(),
            context.robot_kinematics.head.head_to_robot.inverse(),
            head_to_top_camera,
        );

        let head_to_bottom_camera = head_to_camera(
            context.bottom_camera_matrix_parameters.extrinsic_rotations,
            context
                .bottom_camera_matrix_parameters
                .camera_pitch
                .to_radians(),
            RobotDimensions::HEAD_TO_BOTTOM_CAMERA,
        );
        let uncalibrated_bottom_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.bottom_camera_matrix_parameters.focal_lengths,
            context.bottom_camera_matrix_parameters.cc_optical_center,
            image_size,
            context.robot_to_ground.inverse(),
            context.robot_kinematics.head.head_to_robot.inverse(),
            head_to_bottom_camera,
        );

        let correction_in_robot = Rotation3::from_euler_angles(
            context.correction_in_robot.x,
            context.correction_in_robot.y,
            context.correction_in_robot.z,
        );
        let correction_in_camera_top = Rotation3::from_euler_angles(
            context.correction_in_camera_top.x,
            context.correction_in_camera_top.y,
            context.correction_in_camera_top.z,
        );
        let correction_in_camera_bottom = Rotation3::from_euler_angles(
            context.correction_in_camera_bottom.x,
            context.correction_in_camera_bottom.y,
            context.correction_in_camera_bottom.z,
        );

        let calibrated_top_camera_matrix = uncalibrated_top_camera_matrix.to_corrected(
            correction_in_robot,
            correction_in_camera_top,
            None,
        );
        let calibrated_bottom_camera_matrix = uncalibrated_bottom_camera_matrix.to_corrected(
            correction_in_robot,
            correction_in_camera_bottom,
            None,
        );

        let field_dimensions = context.field_dimensions;
        context
            .projected_field_lines
            .fill_if_subscribed(|| ProjectedFieldLines {
                top: project_lines_onto_image(
                    field_dimensions,
                    &calibrated_top_camera_matrix,
                    *context.ground_to_field,
                ),
                bottom: project_lines_onto_image(
                    field_dimensions,
                    &calibrated_bottom_camera_matrix,
                    *context.ground_to_field,
                ),
            });

        Ok(MainOutputs {
            uncalibrated_camera_matrices: Some(CameraMatrices {
                top: uncalibrated_top_camera_matrix,
                bottom: uncalibrated_bottom_camera_matrix,
            })
            .into(),
            camera_matrices: Some(CameraMatrices {
                top: calibrated_top_camera_matrix,
                bottom: calibrated_bottom_camera_matrix,
            })
            .into(),
        })
    }
}

fn project_lines_onto_image(
    field_dimensions: &FieldDimensions,
    camera_matrix: &CameraMatrix,
    ground_to_field: Isometry2<Ground, Field>,
) -> Vec<LineSegment<Pixel>> {
    let field_to_pixel = |line: LineSegment<Field>| -> Option<LineSegment<Pixel>> {
        let field_to_ground = ground_to_field.inverse();
        Some(LineSegment(
            camera_matrix
                .ground_to_pixel(field_to_ground * line.0)
                .ok()?,
            camera_matrix
                .ground_to_pixel(field_to_ground * line.1)
                .ok()?,
        ))
    };
    let field_length = field_dimensions.length;
    let penalty_area_length = field_dimensions.penalty_area_length;
    let penalty_area_width = field_dimensions.penalty_area_width;

    let penalty_top_left = point![field_length / 2.0, penalty_area_width / 2.0];
    let penalty_top_right = point![field_length / 2.0, -penalty_area_width / 2.0];
    let penalty_bottom_left = point![
        field_length / 2.0 - penalty_area_length,
        penalty_area_width / 2.0
    ];
    let penalty_bottom_right = point![
        field_length / 2.0 - penalty_area_length,
        -penalty_area_width / 2.0
    ];

    [
        LineSegment(penalty_bottom_left, penalty_bottom_right),
        LineSegment(penalty_bottom_left, penalty_top_left),
        LineSegment(penalty_bottom_right, penalty_top_right),
        LineSegment(
            field_dimensions.corner(Half::Own, Side::Left),
            field_dimensions.corner(Half::Own, Side::Right),
        ),
        LineSegment(
            field_dimensions.corner(Half::Own, Side::Right),
            field_dimensions.corner(Half::Opponent, Side::Right),
        ),
        LineSegment(
            field_dimensions.corner(Half::Opponent, Side::Right),
            field_dimensions.corner(Half::Opponent, Side::Left),
        ),
        LineSegment(
            field_dimensions.corner(Half::Opponent, Side::Left),
            field_dimensions.corner(Half::Own, Side::Left),
        ),
        LineSegment(
            field_dimensions.t_crossing(Side::Left),
            field_dimensions.t_crossing(Side::Right),
        ),
    ]
    .into_iter()
    .filter_map(field_to_pixel)
    .collect::<Vec<_>>()
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
