use ros_z::topic_name::TopicNameError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    RosZ(#[from] ros_z::Error),

    #[error("invalid topic selector '{selector}': {source}")]
    InvalidTopicSelector {
        selector: String,
        #[source]
        source: TopicNameError,
    },

    #[error(
        "private topic selector '{selector}' requires target node context; debug topic selectors currently resolve against a target namespace only"
    )]
    UnsupportedPrivateTopicSelector { selector: String },

    #[error("invalid target namespace '{target_namespace}': {source}")]
    InvalidTargetNamespace {
        target_namespace: String,
        #[source]
        source: TopicNameError,
    },

    #[error("invalid retention policy: {0}")]
    InvalidRetention(String),
}
