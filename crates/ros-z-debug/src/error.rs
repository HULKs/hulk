use ros_z::topic_name::TopicNameError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid topic reference '{topic}': {source}")]
    InvalidTopicReference {
        topic: String,
        #[source]
        source: TopicNameError,
    },

    #[error("invalid target namespace '{target_namespace}': {source}")]
    InvalidTargetNamespace {
        target_namespace: String,
        #[source]
        source: TopicNameError,
    },

    #[error("invalid target node name '{target_node_name}': {source}")]
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

#[cfg(test)]
mod tests {
    use crate::{TargetIdentity, TopicReference};

    #[test]
    fn invalid_topic_reference_display_includes_validation_cause() {
        let error = TopicReference::new("foo%bar").unwrap_err();

        let message = error.to_string();

        assert!(
            message.starts_with("invalid topic reference 'foo%bar': "),
            "display omitted validation cause: {message}"
        );
        assert!(message.contains("foo%bar"));
    }

    #[test]
    fn invalid_target_namespace_display_includes_validation_cause() {
        let error = TargetIdentity::new("alpha%bad").unwrap_err();

        let message = error.to_string();

        assert!(
            message.starts_with("invalid target namespace 'alpha%bad': "),
            "display omitted validation cause: {message}"
        );
        assert!(message.contains("alpha%bad"));
    }

    #[test]
    fn invalid_target_node_name_display_includes_validation_cause() {
        let error = TargetIdentity::new("/42")
            .unwrap()
            .with_node_name("bad%node")
            .unwrap_err();

        let message = error.to_string();

        assert!(
            message.starts_with("invalid target node name 'bad%node': "),
            "display omitted validation cause: {message}"
        );
        assert!(message.contains("bad%node"));
    }
}
