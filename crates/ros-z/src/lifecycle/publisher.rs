use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tracing::warn;
use zenoh::Result;

use crate::{
    msg::{WireEncoder, WireMessage},
    pubsub::Publisher,
};

/// Trait for entities managed by [`LifecycleNode`](super::node::LifecycleNode).
///
/// Lifecycle publishers implement this trait so the node can bulk-activate or
/// bulk-deactivate all its publishers when a lifecycle transition occurs.
pub trait ManagedEntity: Send + Sync {
    fn on_activate(&self);
    fn on_deactivate(&self);
}

/// A ros-z lifecycle-aware publisher.
///
/// Wraps a [`Publisher`] and silently drops `publish()` calls while the
/// publisher is in the deactivated state. The activated state is toggled by the owning
/// [`LifecycleNode`](super::node::LifecycleNode) via the [`ManagedEntity`] trait.
///
/// # Example
///
/// ```no_run
/// use ros_z::lifecycle::LifecycleNode;
/// use ros_z_msgs::ros::std_msgs::String as RosString;
///
/// # async fn example(node: &mut LifecycleNode) -> zenoh::Result<()> {
/// let pub_ = node.create_publisher::<RosString>("chatter").await?;
/// // pub_ drops messages until the node is activated
/// # Ok(())
/// # }
/// ```
pub struct LifecyclePublisher<T: WireMessage, S: WireEncoder = <T as WireMessage>::Codec> {
    inner: Publisher<T, S>,
    activated: Arc<AtomicBool>,
    /// Throttle "publisher not activated" warnings to one per deactivation cycle.
    should_warn: AtomicBool,
}

impl<T: WireMessage, S: WireEncoder> LifecyclePublisher<T, S> {
    pub(super) fn new(inner: Publisher<T, S>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            activated: Arc::new(AtomicBool::new(false)),
            should_warn: AtomicBool::new(true),
        })
    }

    /// Publish a message. Silently dropped (with one warning) when deactivated.
    pub async fn publish(&self, message: &T) -> Result<()>
    where
        T: 'static,
        S: for<'a> crate::msg::WireEncoder<Input<'a> = &'a T> + 'static,
    {
        if !self.activated.load(Ordering::Relaxed) {
            if self
                .should_warn
                .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                warn!(
                    topic = %self.inner.entity().topic,
                    "publish called while lifecycle publisher is deactivated; message dropped"
                );
            }
            return Ok(());
        }
        self.inner.publish(message).await
    }

    /// Returns `true` if this publisher is currently activated.
    pub fn is_activated(&self) -> bool {
        self.activated.load(Ordering::Relaxed)
    }

    /// The fully-qualified topic name.
    pub fn topic_name(&self) -> &str {
        &self.inner.entity().topic
    }
}

impl<T: WireMessage, S: WireEncoder + Send + Sync> ManagedEntity for LifecyclePublisher<T, S> {
    fn on_activate(&self) {
        self.activated.store(true, Ordering::Relaxed);
        // Re-arm the warning so it fires on the next deactivation cycle.
        self.should_warn.store(true, Ordering::Relaxed);
    }

    fn on_deactivate(&self) {
        self.activated.store(false, Ordering::Relaxed);
    }
}

impl<T: WireMessage + std::fmt::Debug, S: WireEncoder> std::fmt::Debug
    for LifecyclePublisher<T, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecyclePublisher")
            .field("topic", &self.inner.entity().topic)
            .field("activated", &self.activated.load(Ordering::Relaxed))
            .finish()
    }
}
