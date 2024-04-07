use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use approx::assert_relative_eq;

use coordinate_systems::{Camera, Head, Pixel};
use linear_algebra::{point, vector, IntoTransform, Isometry3, Vector2, Vector3};
use projection::{camera_matrix::CameraMatrix, Projection};

fn from_normalized_focal_and_center_short(
    focal_length: nalgebra::Vector2<f32>,
    optical_center: nalgebra::Point2<f32>,
    image_size: Vector2<Pixel>,
) -> CameraMatrix {
    CameraMatrix::from_normalized_focal_and_center(
        focal_length,
        optical_center,
        image_size,
        Isometry3::identity(),
        Isometry3::identity(),
        Isometry3::from_translation(0.0, 0.0, 1.0),
    )
}

fn head_to_camera(camera_pitch: f32, head_to_camera: Vector3<Head>) -> Isometry3<Head, Camera> {
    (nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * -camera_pitch)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * -FRAC_PI_2)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * FRAC_PI_2)
        * nalgebra::Isometry3::from(-head_to_camera.inner))
    .framed_transform()
}

#[test]
fn bearing_projects_back_to_pixel() {
    let camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![640.0, 480.0],
    );

    let pixel = point![32.0, 42.0];
    let bearing = camera_matrix.bearing(pixel);

    assert_relative_eq!(camera_matrix.camera_to_pixel(bearing).unwrap(), pixel);
}

#[test]
fn camera_to_pixel_default_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![0.0, 0.0, 1.0])
            .unwrap(),
        point![1.0, 1.0]
    );
}

#[test]
fn camera_to_pixel_default_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![-0.5, -0.5, 1.0])
            .unwrap(),
        point![0.0, 0.0]
    );
}

#[test]
fn camera_to_pixel_sample_camera_center() {
    let camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![0.95, 1.27],
        nalgebra::point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![0.0, 0.0, 1.0])
            .unwrap(),
        point![320.0, 240.0]
    );
}

#[test]
fn camera_to_pixel_sample_camera_top_left() {
    let camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![0.95, 1.27],
        nalgebra::point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    assert_relative_eq!(
        camera_matrix
            .camera_to_pixel(vector![-0.5 / 0.95, -0.5 / 1.27, 1.0])
            .unwrap(),
        point![0.0, 0.0],
        epsilon = 0.0001
    );
}

#[test]
fn pixel_to_ground_with_z_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );

    camera_matrix.head_to_camera = head_to_camera(0.0, vector![0.0, 0.0, 0.5]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .pixel_to_ground_with_z(point![1.0, 2.0], 0.25)
            .unwrap(),
        point![0.5, 0.0]
    );
}

#[test]
fn pixel_to_ground_from_center_circle() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![0.95, 1.27],
        nalgebra::point![0.5, 0.5],
        vector![640.0, 480.0],
    );
    camera_matrix.head_to_camera = head_to_camera(-1.2_f32.to_radians(), vector![0.0, 0.0, 0.54]);
    camera_matrix.compute_memoized();

    let goal_center = point![4.5, 0.0];
    assert_relative_eq!(
        camera_matrix.ground_to_pixel(goal_center).unwrap(),
        point![320.0, 300.23],
        epsilon = 0.01
    );
}

#[test]
fn pixel_to_ground_with_z_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.head_to_camera = head_to_camera(-FRAC_PI_4, vector![0.0, 0.0, 0.5]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .pixel_to_ground_with_z(point![1.0, 1.0], 0.0)
            .unwrap(),
        point![0.5, 0.0]
    );
}

#[test]
fn pixel_to_ground_with_z_pitch_45_degree_up() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.head_to_camera = head_to_camera(FRAC_PI_4, vector![0.0, 0.0, 0.5]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .pixel_to_ground_with_z(point![1.0, 1.0], 1.0)
            .unwrap(),
        point![0.5, 0.0]
    );
}

#[test]
fn ground_to_pixel_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![0.5, 0.5],
        vector![1.0, 1.0],
    );
    camera_matrix.head_to_camera = head_to_camera(0.0, vector![0.0, 0.0, 0.75]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .ground_with_z_to_pixel(point![1.0, 0.0], 0.25)
            .unwrap(),
        point![0.5, 1.5],
        epsilon = 0.0001,
    );
}

#[test]
fn ground_to_pixel_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![1.0, 1.0],
        nalgebra::point![0.5, 0.5],
        vector![1.0, 1.0],
    );

    camera_matrix.head_to_camera = head_to_camera(-FRAC_PI_4, vector![0.0, 0.0, 1.0]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .ground_with_z_to_pixel(point![0.5, 0.0], 0.5)
            .unwrap(),
        point![0.5, 0.5]
    );
}

#[test]
fn robot_to_pixel_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![2.0, 2.0],
        nalgebra::point![1.0, 1.0],
        vector![1.0, 1.0],
    );
    camera_matrix.head_to_camera = head_to_camera(0.0, vector![0.0, 0.0, 0.75]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .robot_to_pixel(point![1.0, 0.0, 0.25])
            .unwrap(),
        point![1.0, 2.0],
        epsilon = 0.0001,
    );
}

#[test]
fn robot_to_pixel_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![1.0, 1.0],
        nalgebra::point![0.5, 0.5],
        vector![1.0, 1.0],
    );
    camera_matrix.head_to_camera = head_to_camera(-FRAC_PI_4, vector![0.0, 0.0, 1.0]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix.robot_to_pixel(point![0.5, 0.0, 0.5]).unwrap(),
        point![0.5, 0.5]
    );
}

#[test]
fn get_pixel_radius_only_elevation() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![1.0, 1.0],
        nalgebra::point![0.5, 0.5],
        vector![640.0, 480.0],
    );
    camera_matrix.field_of_view = nalgebra::vector![45.0, 45.0].map(|a: f32| a.to_radians());

    camera_matrix.head_to_camera = head_to_camera(0.0, vector![0.0, 0.0, 0.5]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .get_pixel_radius(0.05, point![320.0, 480.0])
            .unwrap(),
        30.34,
        epsilon = 0.01,
    );
}

#[test]
fn get_pixel_radius_pitch_45_degree_down() {
    let mut camera_matrix = from_normalized_focal_and_center_short(
        nalgebra::vector![1.0, 1.0],
        nalgebra::point![0.5, 0.5],
        vector![640.0, 480.0],
    );

    camera_matrix.head_to_camera = head_to_camera(-FRAC_PI_4, vector![0.0, 0.0, 0.5]);
    camera_matrix.compute_memoized();

    assert_relative_eq!(
        camera_matrix
            .get_pixel_radius(0.05, point![320.0, 480.0])
            .unwrap(),
        54.36,
        epsilon = 0.01,
    );
}
