use types::{
    camera_position::CameraPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
};

pub fn execute(enable_pose_detection: bool) -> Option<MotionCommand> {
    Some(MotionCommand::Stand {
        head: if enable_pose_detection {
            HeadMotion::LookAtReferee {
                image_region_target: ImageRegion::Bottom,
                camera: Some(CameraPosition::Top),
            }
        } else {
            HeadMotion::Center
        },
    })
}
