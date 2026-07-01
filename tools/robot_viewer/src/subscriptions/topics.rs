use std::time::Duration;

pub(crate) const CAMERA_IMAGE_TOPIC: &str = "inputs/left_image";

pub(super) const FIELD_DIMENSIONS_TOPIC: &str = "field_dimensions";
pub(super) const LOCALIZATION_TOPIC: &str = "localization";
pub(super) const VISUAL_ODOMETER_TOPIC: &str =
    "visual_odometry/current_left_camera_to_visual_odometer";
pub(super) const ROBOT_KINEMATICS_TOPIC: &str = "robot_kinematics";
pub(super) const CAMERA_MATRIX_TOPIC: &str = "camera_matrix";
pub(super) const CALIBRATED_INTRINSICS_TOPIC: &str = "debug/calibrated_intrinsics";
// Detections are an announced stream: replays/simulators must provide both this base topic and
// `{DETECTED_OBJECTS_TOPIC}/announce` so the viewer can recover the original image timestamp.
pub(super) const DETECTED_OBJECTS_TOPIC: &str = "detected_objects";
pub(super) const FIELD_MARK_ASSOCIATIONS_TOPIC: &str = "field_mark_association/associations";

pub(super) const DETECTED_OBJECTS_SAFETY_LAG: Duration = Duration::from_millis(50);
pub(super) const DEBUG_REFRESH_INTERVAL: Duration = Duration::from_millis(33);
pub(super) const DEBUG_HISTORY_WINDOW: Duration = Duration::from_secs(2);
pub(super) const DEBUG_HIGH_RATE_HISTORY_CAPACITY: usize = 1024;
pub(super) const DEBUG_STREAM_HISTORY_CAPACITY: usize = 64;
pub(super) const PROCESSED_PUBLICATION_CAPACITY: usize = 4096;
