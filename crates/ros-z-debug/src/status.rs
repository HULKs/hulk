use ros_z::TypeInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubscriptionStatus {
    Disconnected,
    Connecting,
    Resolving,
    NoPublishers,
    Subscribing,
    WaitingForFirstSample,
    Ready,
    TypeConflict,
    SchemaUnavailable,
    ProtocolError,
    DecodeError,
    Retargeting,
    Closed,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SubscriptionStatusSnapshot {
    pub status: SubscriptionStatus,
    pub message: Option<String>,
    pub resolved_topic: Option<String>,
    pub type_info: Option<TypeInfo>,
}

impl SubscriptionStatusSnapshot {
    pub fn new(status: SubscriptionStatus) -> Self {
        Self {
            status,
            message: None,
            resolved_topic: None,
            type_info: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{SubscriptionStatus, SubscriptionStatusSnapshot};

    #[test]
    fn snapshot_helpers_set_status_and_message() {
        let snapshot = SubscriptionStatusSnapshot::new(SubscriptionStatus::NoPublishers)
            .with_message("no matching publishers");

        assert_eq!(snapshot.status, SubscriptionStatus::NoPublishers);
        assert_eq!(snapshot.message.as_deref(), Some("no matching publishers"));
        assert_eq!(snapshot.resolved_topic, None);
        assert_eq!(snapshot.type_info, None);
    }
}
