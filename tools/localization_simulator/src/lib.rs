//! Deterministic, headless inputs and runner for exercising 3D localization.

pub mod bevy_scene;
pub mod config;
pub mod production_vo;
pub mod report;
mod sensors;
pub mod simulation;
pub mod stereo_render;
pub mod trajectory;

pub use config::{AssociationMode, SimulationConfig, VisualOdometryMode, VisualOdometryOutlier};
pub use simulation::{
    LandmarkClass, LandmarkDetection, LandmarkFrameCounts, LocalizationSimulation,
    SimulationHistorySample,
};
pub use trajectory::{PoseKeyframe, Scenario};
