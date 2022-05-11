use macros::{module, require_some};
use nalgebra::{point, vector, Isometry3, UnitQuaternion, Vector2, Vector3};

use crate::{
    framework::configuration::CameraMatrixParameters,
    types::{
        CameraMatrices, CameraMatrix, CameraPosition, FieldDimensions, Horizon, Line, Line2,
        ProjectedFieldLines, RobotDimensions, RobotKinematics,
    },
};

#[derive(Default)]
pub struct CameraMatrixProvider;

#[module(control)]
#[input(path = robot_to_ground, data_type = Isometry3<f32>)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(name = top_camera_matrix_parameters, path = vision_top.camera_matrix_parameters, data_type = CameraMatrixParameters)]
#[parameter(name = bottom_camera_matrix_parameters, path = vision_bottom.camera_matrix_parameters, data_type = CameraMatrixParameters)]
#[additional_output(path = projected_field_lines, data_type = ProjectedFieldLines)]
#[main_output(data_type = CameraMatrices)]
impl CameraMatrixProvider {}

impl CameraMatrixProvider {
    pub fn new() -> Self {
        Self
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let robot_kinematics = require_some!(context.robot_kinematics);
        let robot_to_ground = require_some!(context.robot_to_ground);

        let top_camera_to_head = camera_to_head(
            CameraPosition::Top,
            context.top_camera_matrix_parameters.extrinsic_rotations,
        );
        let top_camera_to_robot = robot_kinematics.head_to_robot * top_camera_to_head;
        let top_camera_to_ground = robot_to_ground * top_camera_to_robot;
        let top_camera_matrix = camera_matrix_for_camera(
            top_camera_to_robot,
            top_camera_to_ground,
            context.top_camera_matrix_parameters.focal_lengths,
            context.top_camera_matrix_parameters.cc_optical_center,
        );
        let bottom_camera_to_head = camera_to_head(
            CameraPosition::Bottom,
            context.bottom_camera_matrix_parameters.extrinsic_rotations,
        );
        let bottom_camera_to_robot = robot_kinematics.head_to_robot * bottom_camera_to_head;
        let bottom_camera_to_ground = robot_to_ground * bottom_camera_to_robot;
        let bottom_camera_matrix = camera_matrix_for_camera(
            bottom_camera_to_robot,
            bottom_camera_to_ground,
            context.bottom_camera_matrix_parameters.focal_lengths,
            context.bottom_camera_matrix_parameters.cc_optical_center,
        );

        let field_dimensions = context.field_dimensions;
        context
            .projected_field_lines
            .on_subscription(|| ProjectedFieldLines {
                top: project_penalty_area_on_images(field_dimensions, &top_camera_matrix)
                    .unwrap_or_default(),
                bottom: project_penalty_area_on_images(field_dimensions, &bottom_camera_matrix)
                    .unwrap_or_default(),
            });

        Ok(MainOutputs {
            camera_matrices: Some(CameraMatrices {
                top: top_camera_matrix,
                bottom: bottom_camera_matrix,
            }),
        })
    }
}

pub fn camera_to_head(
    camera_position: CameraPosition,
    extrinsic_rotation: Vector3<f32>,
) -> Isometry3<f32> {
    let extrinsic_angles_in_radians = extrinsic_rotation.map(|a: f32| a.to_radians());
    let extrinsic_rotation = UnitQuaternion::from_euler_angles(
        extrinsic_angles_in_radians.x,
        extrinsic_angles_in_radians.y,
        extrinsic_angles_in_radians.z,
    );
    let neck_to_camera = match camera_position {
        CameraPosition::Top => RobotDimensions::NECK_TO_TOP_CAMERA,
        CameraPosition::Bottom => RobotDimensions::NECK_TO_BOTTOM_CAMERA,
    };
    let camera_pitch = match camera_position {
        CameraPosition::Top => 1.2f32.to_radians(),
        CameraPosition::Bottom => 39.7f32.to_radians(),
    };
    Isometry3::from(neck_to_camera)
        * Isometry3::rotation(Vector3::y() * camera_pitch)
        * extrinsic_rotation
}

fn camera_matrix_for_camera(
    camera_to_robot: Isometry3<f32>,
    camera_to_ground: Isometry3<f32>,
    focal_length: Vector2<f32>,
    optical_center: Vector2<f32>,
) -> CameraMatrix {
    // Calculate FOV using;
    // fov_x = 2 * atan(image_width/ (2 * focal_lengths_x)) -> same for fov_y.
    // https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
    // focal_lengths & cc_optical_center in [0, 1] range & assuming image_size -> 1.0
    let field_of_view = focal_length.map(|f| 2.0 * (0.5 / f).atan());

    let image_width = 640;
    let image_height = 480;
    let focal_length = vector![
        focal_length.x * (image_width as f32),
        focal_length.y * (image_height as f32)
    ];
    let optical_center = point![
        optical_center.x * (image_width as f32),
        optical_center.y * (image_height as f32)
    ];

    let rotation_matrix = camera_to_ground.rotation.to_rotation_matrix();
    let horizon_slope_is_infinite = rotation_matrix[(2, 2)] == 0.0;
    let horizon = if horizon_slope_is_infinite {
        Horizon {
            left_horizon_y: 0.0,
            right_horizon_y: 0.0,
        }
    } else {
        let left_horizon_y = optical_center.y
            + focal_length.y
                * (rotation_matrix[(2, 0)]
                    + optical_center.x * rotation_matrix[(2, 1)] / focal_length.x)
                / rotation_matrix[(2, 2)];
        let slope =
            -focal_length.y * rotation_matrix[(2, 1)] / (focal_length.x * rotation_matrix[(2, 2)]);
        let right_horizon_y = left_horizon_y + (slope * ((image_width - 1) as f32));

        Horizon {
            left_horizon_y,
            right_horizon_y,
        }
    };

    CameraMatrix {
        camera_to_ground,
        ground_to_camera: camera_to_ground.inverse(),
        camera_to_robot,
        robot_to_camera: camera_to_robot.inverse(),
        focal_length,
        optical_center,
        field_of_view,
        horizon,
    }
}

fn project_penalty_area_on_images(
    field_dimensions: &FieldDimensions,
    camera_matrix: &CameraMatrix,
) -> Option<Vec<Line2>> {
    let field_length = &field_dimensions.length;
    let field_width = &field_dimensions.width;
    let penalty_area_length = &field_dimensions.penalty_area_length;
    let penalty_area_width = &field_dimensions.penalty_area_width;

    let penalty_top_left = camera_matrix
        .ground_to_pixel(&point![field_length / 2.0, penalty_area_width / 2.0])
        .ok()?;
    let penalty_top_right = camera_matrix
        .ground_to_pixel(&point![field_length / 2.0, -penalty_area_width / 2.0])
        .ok()?;
    let penalty_bottom_left = camera_matrix
        .ground_to_pixel(&point![
            field_length / 2.0 - penalty_area_length,
            penalty_area_width / 2.0
        ])
        .ok()?;
    let penalty_bottom_right = camera_matrix
        .ground_to_pixel(&point![
            field_length / 2.0 - penalty_area_length,
            -penalty_area_width / 2.0
        ])
        .ok()?;
    let corner_left = camera_matrix
        .ground_to_pixel(&point![field_length / 2.0, field_width / 2.0])
        .ok()?;
    let corner_right = camera_matrix
        .ground_to_pixel(&point![field_length / 2.0, -field_width / 2.0])
        .ok()?;

    Some(vec![
        Line(penalty_top_left, penalty_top_right),
        Line(penalty_bottom_left, penalty_bottom_right),
        Line(penalty_bottom_left, penalty_top_left),
        Line(penalty_bottom_right, penalty_top_right),
        Line(corner_left, corner_right),
    ])
}
