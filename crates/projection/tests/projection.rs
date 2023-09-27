use approx::assert_relative_eq;
use nalgebra::{point, vector, Isometry3, Point2, Translation, UnitQuaternion, Vector2};
use projection::Projection;
use types::camera_matrix::CameraMatrix;

fn from_normalized_focal_and_center_short(
    focal_length: Vector2<f32>,
    optical_center: Point2<f32>,
    image_size: Vector2<f32>,
) -> CameraMatrix {
    CameraMatrix::from_normalized_focal_and_center(
        focal_length,
        optical_center,
        image_size,
        Isometry3::identity(),
        Isometry3::identity(),
        Isometry3::identity(),
    )
}

#[test]
fn pixel_to_camera_default_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    assert_relative_eq!(
        camera_matrix.pixel_to_camera(point![1.0, 1.0]),
        vector![1.0, 0.0, 0.0]
    );
}

#[test]
fn pixel_to_camera_default_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![2.0, 2.0],
    );

    assert_relative_eq!(
        camera_matrix.pixel_to_camera(point![0.0, 0.0]),
        vector![1.0, 0.5, 0.5]
    );
}

#[test]
fn pixel_to_camera_sample_camera_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![0.95, 1.27],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix.pixel_to_camera(point![320.0, 240.0]),
        vector![1.0, 0.0, 0.0]
    );
}

#[test]
fn pixel_to_camera_sample_camera_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![0.95, 1.27],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix.pixel_to_camera(point![0.0, 0.0]),
        vector![1.0, 0.5 / 0.95, 0.5 / 1.27]
    );
}

#[test]
fn camera_to_pixel_default_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![1.0, 0.0, 0.0])
            .unwrap(),
        point![1.0, 1.0]
    );
}

#[test]
fn camera_to_pixel_default_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![1.0, 0.5, 0.5])
            .unwrap(),
        point![0.0, 0.0]
    );
}

#[test]
fn camera_to_pixel_sample_camera_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![0.95, 1.27],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![1.0, 0.0, 0.0])
            .unwrap(),
        point![320.0, 240.0]
    );
}

#[test]
fn camera_to_pixel_sample_camera_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![0.95, 1.27],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![1.0, 0.5 / 0.95, 0.5 / 1.27])
            .unwrap(),
        point![0.0, 0.0],
        epsilon = 0.0001
    );
}

#[test]
fn pixel_to_camera_reversible() {
    let camera_matrix = from_normalized_focal_and_center_short(
        vector![0.95, 1.27],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    let input = point![512.0, 257.0];
    let output = camera_matrix
        .camera_to_pixel(camera_matrix.pixel_to_camera(input))
        .unwrap();

    assert_relative_eq!(input, output);
}

#[test]
fn pixel_to_ground_with_z_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);

    assert_relative_eq!(
        camera_matrix
            .pixel_to_ground_with_z(point![1.0, 2.0], 0.25)
            .unwrap(),
        point![0.5, 0.0]
    );
}

#[test]
fn pixel_to_ground_with_z_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);
    camera_matrix.camera_to_ground.rotation =
        UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);

    assert_relative_eq!(
        camera_matrix
            .pixel_to_ground_with_z(point![1.0, 1.0], 0.0)
            .unwrap(),
        point![0.5, 0.0]
    );
}

#[test]
fn ground_to_pixel_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.75]);
    camera_matrix.ground_to_camera = camera_matrix.camera_to_ground.inverse();

    assert_relative_eq!(
        camera_matrix
            .ground_with_z_to_pixel(point![1.0, 0.0], 0.25)
            .unwrap(),
        point![1.0, 2.0]
    );
}

#[test]
fn ground_to_pixel_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![1.0, 1.0],
        point![0.5, 0.5],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 1.0]);
    camera_matrix.camera_to_ground.rotation =
        UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);
    camera_matrix.ground_to_camera = camera_matrix.camera_to_ground.inverse();

    assert_relative_eq!(
        camera_matrix
            .ground_with_z_to_pixel(point![0.5, 0.0], 0.5)
            .unwrap(),
        point![0.5, 0.5]
    );
}

#[test]
fn pixel_to_robot_with_x() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 0.75]);
    camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

    let pixel_coordinates = point![1.5, 2.0];
    let robot_coordinates = camera_matrix
        .pixel_to_robot_with_x(pixel_coordinates, 0.5)
        .unwrap();
    assert_relative_eq!(robot_coordinates, point![0.5, -0.125, 0.5]);
}

#[test]
fn robot_to_pixel_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 0.75]);
    camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

    assert_relative_eq!(
        camera_matrix
            .robot_to_pixel(point![1.0, 0.0, 0.25])
            .unwrap(),
        point![1.0, 2.0]
    );
}

#[test]
fn robot_to_pixel_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![1.0, 1.0],
        point![0.5, 0.5],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 1.0]);
    camera_matrix.camera_to_robot.rotation =
        UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);
    camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

    assert_relative_eq!(
        camera_matrix.robot_to_pixel(point![0.5, 0.0, 0.5]).unwrap(),
        point![0.5, 0.5]
    );
}

#[test]
fn robot_to_pixel_inverse() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 0.75]);
    camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

    let robot_coordinates = point![1.0, 2.0, 1.0];
    let pixel_coordinates = camera_matrix.robot_to_pixel(robot_coordinates).unwrap();
    assert_relative_eq!(
        camera_matrix
            .pixel_to_robot_with_x(pixel_coordinates, robot_coordinates.x)
            .unwrap(),
        robot_coordinates
    );
}

#[test]
fn pixel_to_robot_with_x_inverse() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![2.0, 2.0],
        point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 0.75]);
    camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

    let pixel_coordinates = point![0.75, 2.0];
    let robot_coordinates = camera_matrix
        .pixel_to_robot_with_x(pixel_coordinates, 0.5)
        .unwrap();
    assert_relative_eq!(
        camera_matrix.robot_to_pixel(robot_coordinates).unwrap(),
        pixel_coordinates
    );
}

#[test]
fn get_pixel_radius_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![1.0, 1.0],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );
    camera_matrix.field_of_view = vector![45.0, 45.0].map(|a: f32| a.to_radians());
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);

    assert_relative_eq!(
        camera_matrix
            .get_pixel_radius(0.05, point![320.0, 480.0], vector![640, 480])
            .unwrap(),
        33.970547
    );
}

#[test]
fn get_pixel_radius_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        vector![1.0, 1.0],
        point![0.5, 0.5],
        vector![640.0, 480.0],
    );
    camera_matrix.field_of_view = vector![45.0, 45.0].map(|a: f32| a.to_radians());
    camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);
    camera_matrix.camera_to_ground.rotation =
        UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);

    assert_relative_eq!(
        camera_matrix
            .get_pixel_radius(0.05, point![320.0, 480.0], vector![640, 480])
            .unwrap(),
        207.69307
    );
}
