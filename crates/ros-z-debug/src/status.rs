use ros_z::TypeInfo;

/// Current lifecycle state for a debug subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubscriptionStatus {
    /// The subscription has been created and is waiting for its first sample.
    WaitingForFirstSample,
    /// At least one sample has been received and retained.
    Ready,
    /// A transport or sample metadata error prevented normal receive handling.
    ProtocolError { message: String },
    /// A received payload could not be decoded as the expected type.
    DecodeError { message: String },
    /// The subscription was closed and will not accept further samples or errors.
    Closed,
}

impl SubscriptionStatus {
    /// Create a protocol error status with a diagnostic message.
    pub fn protocol_error(message: impl Into<String>) -> Self {
        Self::ProtocolError {
            message: message.into(),
        }
    }

    /// Create a decode error status with a diagnostic message.
    pub fn decode_error(message: impl Into<String>) -> Self {
        Self::DecodeError {
            message: message.into(),
        }
    }

    /// Return the diagnostic message for error states.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::ProtocolError { message } | Self::DecodeError { message } => Some(message),
            Self::WaitingForFirstSample | Self::Ready | Self::Closed => None,
        }
    }
}

/// Point-in-time subscription status and resolved metadata.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SubscriptionStatusSnapshot {
    status: SubscriptionStatus,
    resolved_topic: Option<String>,
    type_info: Option<TypeInfo>,
}

impl SubscriptionStatusSnapshot {
    #[cfg(test)]
    pub(crate) fn new(status: SubscriptionStatus) -> Self {
        Self {
            status,
            resolved_topic: None,
            type_info: None,
        }
    }

    pub(crate) fn with_metadata(
        status: SubscriptionStatus,
        resolved_topic: String,
        type_info: TypeInfo,
    ) -> Self {
        Self {
            status,
            resolved_topic: Some(resolved_topic),
            type_info: Some(type_info),
        }
    }

    /// Return the lifecycle status at the time this snapshot was taken.
    pub fn status(&self) -> &SubscriptionStatus {
        &self.status
    }

    /// Return the diagnostic message for error states.
    pub fn message(&self) -> Option<&str> {
        self.status.message()
    }

    /// Return the resolved absolute topic once subscription metadata is known.
    pub fn resolved_topic(&self) -> Option<&str> {
        self.resolved_topic.as_deref()
    }

    /// Return the resolved type metadata once subscription metadata is known.
    pub fn type_info(&self) -> Option<&TypeInfo> {
        self.type_info.as_ref()
    }

    pub(crate) fn set_status(&mut self, status: SubscriptionStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::{SubscriptionStatus, SubscriptionStatusSnapshot};

    #[test]
    fn error_status_carries_message_in_variant() {
        let snapshot = SubscriptionStatusSnapshot::new(SubscriptionStatus::decode_error(
            "failed to decode sample",
        ));

        assert_eq!(snapshot.message(), Some("failed to decode sample"));
        assert_eq!(snapshot.resolved_topic(), None);
        assert_eq!(snapshot.type_info(), None);
    }
}
