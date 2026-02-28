use std::{path::Path, time::Duration};

use source_analyzer::{
    cyclers::{CyclerKind, Cyclers},
    error::Error,
    manifest::{CyclerManifest, FrameworkManifest},
};

pub fn collect_hulk_cyclers(root: impl AsRef<Path>) -> Result<Cyclers, Error> {
    let manifest = FrameworkManifest {
        cyclers: vec![
            CyclerManifest {
                name: "ObjectDetection",
                kind: CyclerKind::Perception,
                instances: vec![""],
                setup_nodes: vec!["object_detection::image_receiver"],
                nodes: vec![
                    "object_detection::object_detection",
                    // "object_detection::pose_detection",
                    // "object_detection::pose_filter",
                    // "object_detection::pose_interpretation",
                ],
                execution_time_warning_threshold: Some(Duration::from_secs_f32(33.0)),
            },
            CyclerManifest {
                name: "Motion",
                kind: CyclerKind::Perception,
                instances: vec![""],
                setup_nodes: vec!["motion::sensor_data_receiver"],
                nodes: vec![
                    "motion::command_sender",
                    "motion::booster_walking",
                    "motion::remote_control",
                    "motion::look_at",
                    "motion::head_motion",
                    "motion::motion_selector",
                    "motion::motor_commands_collector",
                ],
                execution_time_warning_threshold: Some(Duration::from_secs_f32(1.0 / 500.0)),
            },
            CyclerManifest {
                name: "HslNetwork",
                kind: CyclerKind::Perception,
                instances: vec![""],
                setup_nodes: vec!["hsl_network::message_receiver"],
                nodes: vec!["hsl_network::message_filter"],
                execution_time_warning_threshold: None,
            },
            CyclerManifest {
                name: "WorldState",
                kind: CyclerKind::RealTime,
                instances: vec![""],
                setup_nodes: vec!["world_state::trigger"],
                nodes: vec![
                    "world_state::ball_filter",
                    "world_state::ball_projector",
                    "world_state::behavior::node",
                    "world_state::camera_matrix_calculator",
                    "world_state::game_controller_filter",
                    "world_state::game_controller_state_filter",
                    "world_state::ground_provider",
                    "world_state::kinematics_provider",
                    "world_state::primary_state_filter",
                ],
                execution_time_warning_threshold: Some(Duration::from_secs_f32(1.0 / 100.0)),
            },
            // CyclerManifest {
            //     name: "Audio",
            //     kind: CyclerKind::Perception,
            //     instances: vec![""],
            //     setup_nodes: vec!["audio::microphone_recorder"],
            //     nodes: vec!["audio::whistle_detection"],
            //     execution_time_warning_threshold: None,
            // },
            CyclerManifest {
                name: "Image",
                kind: CyclerKind::Perception,
                instances: vec!["Rectified", "StereonetDepth"],
                setup_nodes: vec!["sensor_receiver::image_receiver"],
                nodes: vec![],
                execution_time_warning_threshold: None,
            },
            CyclerManifest {
                name: "FallDownState",
                kind: CyclerKind::Perception,
                instances: vec![""],
                setup_nodes: vec!["sensor_receiver::fall_down_state_receiver"],
                nodes: vec![],
                execution_time_warning_threshold: None,
            },
        ],
    };

    Cyclers::try_from_manifest(manifest, root)
}
