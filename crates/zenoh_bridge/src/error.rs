#[derive(Debug, thiserror::Error)]
pub enum Error {
    // r2r exposes generic error shapes here; keep this variant source-less until the bridge has a concrete ROS error type.
    #[error("ROS error")]
    Ros,
    #[error("Zenoh error: {0}")]
    Zenoh(#[from] zenoh::Error),
}
