use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("ros-z error: {0}")]
    RosZ(String),

    #[error("invalid topic selector '{selector}': {reason}")]
    InvalidTopicSelector { selector: String, reason: String },

    #[error("invalid retention policy: {0}")]
    InvalidRetention(String),
}

impl From<zenoh::Error> for Error {
    fn from(error: zenoh::Error) -> Self {
        Self::RosZ(error.to_string())
    }
}
