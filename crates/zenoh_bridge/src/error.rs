#[derive(Debug, thiserror::Error)]
pub enum Error {
    // FIXME: Internal ROS error can not be wrapped due to generics
    #[error("ROS error")]
    Ros,
    #[error("Zenoh error: {0}")]
    Zenoh(#[from] zenoh::Error),
}
