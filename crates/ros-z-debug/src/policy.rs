use std::{num::NonZeroUsize, time::Duration};

use ros_z::{pubsub::QueueOverflowReporting, qos::QosProfile};

use crate::{Result, RetentionPolicy};

const DEFAULT_UPDATE_BUFFER_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObservationPolicy {
    retention: RetentionPolicy,
    subscriber_qos: Option<QosProfile>,
    subscriber_queue_capacity: Option<NonZeroUsize>,
    update_buffer_capacity: NonZeroUsize,
    queue_overflow_reporting: QueueOverflowReporting,
}

impl Default for ObservationPolicy {
    fn default() -> Self {
        Self {
            retention: RetentionPolicy::LatestOnly,
            subscriber_qos: None,
            subscriber_queue_capacity: None,
            update_buffer_capacity: default_update_buffer_capacity(),
            queue_overflow_reporting: QueueOverflowReporting::Warn,
        }
    }
}

impl ObservationPolicy {
    pub fn latest() -> Self {
        Self {
            queue_overflow_reporting: QueueOverflowReporting::Debug,
            ..Self::default()
        }
    }

    pub fn time_window(duration: Duration) -> Result<Self> {
        Ok(Self {
            retention: RetentionPolicy::time_window(duration)?,
            queue_overflow_reporting: QueueOverflowReporting::Warn,
            ..Self::default()
        })
    }

    pub fn retention(self) -> RetentionPolicy {
        self.retention
    }

    pub fn subscriber_qos(self) -> Option<QosProfile> {
        self.subscriber_qos
    }

    pub fn subscriber_queue_capacity(self) -> Option<NonZeroUsize> {
        self.subscriber_queue_capacity
    }

    pub fn update_buffer_capacity(self) -> NonZeroUsize {
        self.update_buffer_capacity
    }

    pub fn queue_overflow_reporting(self) -> QueueOverflowReporting {
        self.queue_overflow_reporting
    }

    pub fn with_retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    pub fn with_subscriber_qos(mut self, qos: QosProfile) -> Self {
        self.subscriber_qos = Some(qos);
        self
    }

    pub fn with_default_subscriber_qos(mut self) -> Self {
        self.subscriber_qos = None;
        self
    }

    pub fn with_subscriber_queue_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.subscriber_queue_capacity = Some(capacity);
        self
    }

    pub fn derive_subscriber_queue_capacity_from_qos(mut self) -> Self {
        self.subscriber_queue_capacity = None;
        self
    }

    pub fn with_update_buffer_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.update_buffer_capacity = capacity;
        self
    }

    pub fn with_queue_overflow_reporting(mut self, reporting: QueueOverflowReporting) -> Self {
        self.queue_overflow_reporting = reporting;
        self
    }
}

fn default_update_buffer_capacity() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_UPDATE_BUFFER_CAPACITY)
        .expect("default update buffer capacity must be non-zero")
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, time::Duration};

    use ros_z::pubsub::QueueOverflowReporting;

    use crate::{ObservationPolicy, RetentionPolicy};

    #[test]
    fn default_policy_preserves_current_behavior() {
        let policy = ObservationPolicy::default();

        assert_eq!(policy.retention(), RetentionPolicy::LatestOnly);
        assert!(policy.subscriber_qos().is_none());
        assert!(policy.subscriber_queue_capacity().is_none());
        assert_eq!(policy.update_buffer_capacity().get(), 256);
        assert_eq!(
            policy.queue_overflow_reporting(),
            QueueOverflowReporting::Warn
        );
    }

    #[test]
    fn latest_policy_keeps_capacity_derived_from_qos_but_reports_at_debug() {
        let policy = ObservationPolicy::latest();

        assert_eq!(policy.retention(), RetentionPolicy::LatestOnly);
        assert!(policy.subscriber_qos().is_none());
        assert!(policy.subscriber_queue_capacity().is_none());
        assert_eq!(policy.update_buffer_capacity().get(), 256);
        assert_eq!(
            policy.queue_overflow_reporting(),
            QueueOverflowReporting::Debug
        );
    }

    #[test]
    fn time_window_policy_uses_warn_reporting() {
        let policy = ObservationPolicy::time_window(Duration::from_secs(5)).unwrap();

        assert!(matches!(policy.retention(), RetentionPolicy::TimeWindow(_)));
        assert!(policy.subscriber_qos().is_none());
        assert!(policy.subscriber_queue_capacity().is_none());
        assert_eq!(policy.update_buffer_capacity().get(), 256);
        assert_eq!(
            policy.queue_overflow_reporting(),
            QueueOverflowReporting::Warn
        );
    }

    #[test]
    fn overrides_are_permissive_and_independent() {
        let qos = ros_z::qos::QosProfile {
            reliability: ros_z::qos::QosReliability::BestEffort,
            ..Default::default()
        };
        let queue_capacity = NonZeroUsize::new(7).unwrap();
        let update_capacity = NonZeroUsize::new(17).unwrap();

        let policy = ObservationPolicy::latest()
            .with_subscriber_qos(qos)
            .with_subscriber_queue_capacity(queue_capacity)
            .with_update_buffer_capacity(update_capacity)
            .with_queue_overflow_reporting(QueueOverflowReporting::Silent);

        assert_eq!(policy.subscriber_qos(), Some(qos));
        assert_eq!(policy.subscriber_queue_capacity(), Some(queue_capacity));
        assert_eq!(policy.update_buffer_capacity(), update_capacity);
        assert_eq!(
            policy.queue_overflow_reporting(),
            QueueOverflowReporting::Silent
        );
    }
}
