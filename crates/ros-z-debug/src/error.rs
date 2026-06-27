use ros_z::topic_name::TopicNameError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid topic reference '{topic}'")]
    InvalidTopicReference {
        topic: String,
        #[source]
        source: TopicNameError,
    },

    #[error("invalid target namespace '{target_namespace}'")]
    InvalidTargetNamespace {
        target_namespace: String,
        #[source]
        source: TopicNameError,
    },

    #[error("invalid target node name '{target_node_name}'")]
    InvalidTargetNodeName {
        target_node_name: String,
        #[source]
        source: TopicNameError,
    },

    #[error("private topic reference '{topic}' requires a target node name")]
    MissingTargetNodeName { topic: String },

    #[error("invalid retention policy: {0}")]
    InvalidRetention(String),

    #[error(transparent)]
    RosZ(#[from] ros_z::Error),
}
