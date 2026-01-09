use std::f32::consts::FRAC_PI_2;

use color_eyre::Result;
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Ground, Head, Robot};
use framework::MainOutput;
use linear_algebra::{vector, IntoTransform, Isometry3, Rotation3, Vector3};
use types::{
    parameters::CameraMatrixParameters, robot_dimensions::RobotDimensions,
    robot_kinematics::RobotKinematics,
};

#[derive(Deserialize, Serialize)]
pub struct CameraMatrixCalculator {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    // projected_field_lines: AdditionalOutput<Option<ProjectedFieldLines>, "projected_field_lines">,
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
    // ground_to_field: CyclerState<Option<Isometry2<Ground, Field>>, "ground_to_field">,
    // field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.intrinsics">,
    correction_in_robot: Parameter<
        nalgebra::Vector3<f32>,
        "camera_matrix_parameters.extrinsics.correction_in_robot",
    >,
    correction_in_camera: Parameter<
        nalgebra::Vector3<f32>,
        "camera_matrix_parameters.extrinsics.correction_in_camera",
    >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub uncalibrated_camera_matrix: MainOutput<Option<CameraMatrix>>,
    pub camera_matrix: MainOutput<Option<CameraMatrix>>,
}

impl CameraMatrixCalculator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let image_size = vector![640.0, 480.0];
        let head_to_camera = head_to_camera(
            context.camera_matrix_parameters.camera_pitch.to_radians(),
            RobotDimensions::HEAD_TO_CAMERA,
        );
        let uncalibrated_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.camera_matrix_parameters.focal_lengths,
            context.camera_matrix_parameters.cc_optical_center,
            image_size,
            context.robot_to_ground.inverse(),
            context.robot_kinematics.head.head_to_robot.inverse(),
            head_to_camera,
        );

        let correction_in_robot = Rotation3::from_euler_angles(
            context.correction_in_robot.x,
            context.correction_in_robot.y,
            context.correction_in_robot.z,
        );
        let correction_in_camera = Rotation3::from_euler_angles(
            context.correction_in_camera.x,
            context.correction_in_camera.y,
            context.correction_in_camera.z,
        );

        let calibrated_camera_matrix =
            uncalibrated_camera_matrix.to_corrected(correction_in_robot, correction_in_camera);

        // let field_dimensions = context.field_dimensions;
        // context.projected_field_lines.fill_if_subscribed(|| {
        //     context
        //         .ground_to_field
        //         .map(|ground_to_field| ProjectedFieldLines {
        //             field_lines: project_lines_onto_image(
        //                 field_dimensions,
        //                 &calibrated_camera_matrix,
        //                 ground_to_field,
        //             ),
        //         })
        // });

        Ok(MainOutputs {
            uncalibrated_camera_matrix: Some(uncalibrated_camera_matrix).into(),
            camera_matrix: Some(calibrated_camera_matrix).into(),
        })
    }
}

// fn project_lines_onto_image(
//     field_dimensions: &FieldDimensions,
//     camera_matrix: &CameraMatrix,
//     ground_to_field: Isometry2<Ground, Field>,
// ) -> Vec<LineSegment<Pixel>> {
//     let field_to_pixel = |line: LineSegment<Field>| -> Option<LineSegment<Pixel>> {
//         let field_to_ground = ground_to_field.inverse();
//         Some(LineSegment(
//             camera_matrix
//                 .ground_to_pixel(field_to_ground * line.0)
//                 .ok()?,
//             camera_matrix
//                 .ground_to_pixel(field_to_ground * line.1)
//                 .ok()?,
//         ))
//     };
//     let field_length = field_dimensions.length;
//     let penalty_area_length = field_dimensions.penalty_area_length;
//     let penalty_area_width = field_dimensions.penalty_area_width;

//     let penalty_top_left = point![field_length / 2.0, penalty_area_width / 2.0];
//     let penalty_top_right = point![field_length / 2.0, -penalty_area_width / 2.0];
//     let penalty_bottom_left = point![
//         field_length / 2.0 - penalty_area_length,
//         penalty_area_width / 2.0
//     ];
//     let penalty_bottom_right = point![
//         field_length / 2.0 - penalty_area_length,
//         -penalty_area_width / 2.0
//     ];

//     [
//         LineSegment(penalty_bottom_left, penalty_bottom_right),
//         LineSegment(penalty_bottom_left, penalty_top_left),
//         LineSegment(penalty_bottom_right, penalty_top_right),
//         LineSegment(
//             field_dimensions.corner(Half::Own, Side::Left),
//             field_dimensions.corner(Half::Own, Side::Right),
//         ),
//         LineSegment(
//             field_dimensions.corner(Half::Own, Side::Right),
//             field_dimensions.corner(Half::Opponent, Side::Right),
//         ),
//         LineSegment(
//             field_dimensions.corner(Half::Opponent, Side::Right),
//             field_dimensions.corner(Half::Opponent, Side::Left),
//         ),
//         LineSegment(
//             field_dimensions.corner(Half::Opponent, Side::Left),
//             field_dimensions.corner(Half::Own, Side::Left),
//         ),
//         LineSegment(
//             field_dimensions.t_crossing(Side::Left),
//             field_dimensions.t_crossing(Side::Right),
//         ),
//     ]
//     .into_iter()
//     .filter_map(field_to_pixel)
//     .collect::<Vec<_>>()
// }

fn head_to_camera(camera_pitch: f32, head_to_camera: Vector3<Head>) -> Isometry3<Head, Camera> {
    (nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * -camera_pitch)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * -FRAC_PI_2)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * FRAC_PI_2)
        * nalgebra::Isometry3::from(-head_to_camera.inner))
    .framed_transform()
}
