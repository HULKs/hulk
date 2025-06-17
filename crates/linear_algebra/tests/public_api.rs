use linear_algebra::{point, vector, Isometry2, Orientation2, Point2, Pose2, Vector2};

struct Camera;
struct Robot;
struct World;
struct Field;

#[test]
fn macros_construct_framed_types_without_importing_nalgebra() {
    let point_in_world: Point2<World> = point![1.0f32, 2.0];
    let vector_in_world: Vector2<World> = vector![3.0f32, 4.0];

    assert_eq!(point_in_world.x(), 1.0);
    assert_eq!(point_in_world.y(), 2.0);
    assert_eq!(vector_in_world.x(), 3.0);
    assert_eq!(vector_in_world.y(), 4.0);
}

#[test]
fn pose2_new_round_trips_through_as_transform() {
    let pose = Pose2::<World>::new(point![1.0f32, 2.0], 0.5);

    let robot_to_world: Isometry2<Robot, World> = pose.as_transform();
    let reconstructed_pose = robot_to_world.as_pose();

    assert_eq!(reconstructed_pose.position().x(), 1.0);
    assert_eq!(reconstructed_pose.position().y(), 2.0);
    assert_eq!(reconstructed_pose.angle(), 0.5);
}

#[test]
fn pose2_from_parts_preserves_position_and_orientation() {
    let pose = Pose2::<World>::from_parts(point![1.5f32, -0.5], Orientation2::new(0.25));

    assert_eq!(pose.position().x(), 1.5);
    assert_eq!(pose.position().y(), -0.5);
    assert_eq!(pose.orientation().angle(), 0.25);
}

#[test]
fn isometry2_new_preserves_translation_and_angle_via_as_pose() {
    let robot_to_world = Isometry2::<Robot, World>::new(vector![2.0f32, -1.0], 0.5);
    let robot_pose = robot_to_world.as_pose();

    assert_eq!(robot_pose.position().x(), 2.0);
    assert_eq!(robot_pose.position().y(), -1.0);
    assert_eq!(robot_pose.angle(), 0.5);
}

#[test]
fn isometry2_from_parts_preserves_translation_and_orientation() {
    let robot_to_world =
        Isometry2::<Robot, World>::from_parts(vector![1.0f32, 2.0], Orientation2::new(0.25));
    let robot_pose = robot_to_world.as_pose();

    assert_eq!(robot_pose.position().x(), 1.0);
    assert_eq!(robot_pose.position().y(), 2.0);
    assert_eq!(robot_pose.orientation().angle(), 0.25);
}

#[test]
fn composing_transforms_round_trips_points_across_frames() {
    let camera_to_robot = Isometry2::<Camera, Robot>::new(vector![1.0f32, 0.0], 0.0);
    let robot_to_field = Isometry2::<Robot, Field>::new(vector![0.0f32, 3.0], 0.0);
    let camera_to_field = robot_to_field * camera_to_robot;

    let point_in_camera: Point2<Camera> = point![2.0f32, -1.0];
    let point_in_field: Point2<Field> = camera_to_field * point_in_camera;

    assert_eq!(point_in_field.x(), 3.0);
    assert_eq!(point_in_field.y(), 2.0);

    let point_back_in_camera: Point2<Camera> = camera_to_field.inverse() * point_in_field;

    assert_eq!(point_back_in_camera.x(), 2.0);
    assert_eq!(point_back_in_camera.y(), -1.0);
}
