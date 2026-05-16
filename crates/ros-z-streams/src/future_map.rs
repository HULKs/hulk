use std::collections::BTreeMap;
use std::time::Duration;

use ros_z::{
    Message, Result,
    node::Node,
    time::{Clock, Time},
};
use tokio::select;

use crate::future_queue::{CreateFutureQueue, FutureQueueSubscriber, QueueEvent};

/// Type alias for a time-ordered map of buffered data.
///
/// This is the primary data structure used internally to hold messages
/// indexed by their logical timestamp. Messages are held in a `BTreeMap`
/// to maintain strict time ordering and enable efficient binary searches
/// for the global safe time boundary.
pub type FutureResult<Types> = BTreeMap<Time, Types>;

/// A split view of buffered data: persistent (finalized) and temporary (pending).
///
/// When [`FutureMap::recv`] returns, it provides this structure containing two parts:
/// - `persistent`: Data that has crossed the global safe time boundary and will never be
///   reordered. This data is strictly time-ordered and safe for consumption.
/// - `temporary`: A reference to data that may still be reordered as new announcements arrive.
///   These are messages that arrived but whose safe-time boundaries haven't been reached yet.
#[derive(Debug)]
pub struct FutureItem<'a, Types> {
    /// Time-ordered data that has achieved global safety and will never be reordered.
    pub persistent: FutureResult<Types>,
    /// Reference to data still in the temporary buffer, pending finalization.
    pub temporary: &'a FutureResult<Types>,
}

/// Events that can occur when receiving from a stream group.
///
/// Announcements and data messages are processed separately to enable
/// fine-grained buffer management and timing boundary calculations.
#[derive(Debug, PartialEq, Eq)]
pub enum GroupEvent {
    /// A new timestamp announcement was received for a future message.
    Announcement,
    /// Actual data message was received and buffered.
    Data,
}

/// Abstracts multi-stream operations for generic fusion engines.
///
/// This trait allows the `FutureMap` to work with tuples of streams of varying
/// arity (1 to 5 streams). It provides the core operations needed to synchronize
/// multiple asynchronous streams:
/// - Computing the global safe timestamp from all streams
/// - Receiving and buffering events from any stream
/// - Tracking maximum expected transit delays
pub trait StreamGroup {
    /// The output type produced by this group of streams.
    ///
    /// For a 1-stream group of `Type1`, this is `Option<Type1>`.
    /// For a 3-stream group, this is `(Option<Type1>, Option<Type2>, Option<Type3>)`.
    /// The `Option` wrapper allows partial messages (e.g., when only some streams have data at a given time).
    type Output: Default;

    /// Calculates the globally safe timestamp for this group at wall-clock time `now`.
    ///
    /// Returns the minimum of the earliest announced time and the transit safety boundary
    /// across all streams. Data before this timestamp is guaranteed not to be reordered.
    fn global_safe_time(&self, now: Time) -> Time;

    /// Returns the maximum expected transit delay across all streams in this group.
    ///
    /// This is the longest safety lag among all subscribers, used to determine how long
    /// the fusion engine should wait for delayed announcements before finalizing boundaries.
    fn max_safety_lag(&self) -> Option<Duration>;

    /// Receives and buffers the next event from any stream in the group.
    ///
    /// This method is called by the fusion engine to pull events from all streams.
    /// It uses `tokio::select!` internally to multiplex across all stream channels,
    /// updating the provided buffer with new data or announcements as appropriate.
    fn receive_event(
        &mut self,
        buffer: &mut BTreeMap<Time, Self::Output>,
    ) -> impl Future<Output = Result<GroupEvent>>;
}

/// Multi-stream fusion engine that produces time-ordered data splits.
///
/// The `FutureMap` is the core component of this crate. It holds data from multiple
/// subscriptions in a temporal buffer and releases data in strictly time-ordered chunks
/// once the global safe time boundary is reached. This ensures that even with out-of-order
/// delivery and variable latencies, fused data is perfectly synchronized.
///
/// Data is automatically split into two parts on each `recv()` call:
/// - `persistent`: Already-safe, finalized data that will never change
/// - `temporary`: Still-pending data that may be reordered as announcements arrive
#[derive(Debug)]
pub struct FutureMap<Group: StreamGroup> {
    subscribers: Group,
    buffer: FutureResult<Group::Output>,
    clock: Clock,
}

impl<Group: StreamGroup> FutureMap<Group> {
    /// Receives the next batch of time-ordered data from all subscribed streams.
    ///
    /// This method blocks until either:
    /// 1. New persistent (safe) data is available, or
    /// 2. The maximum safety lag timer expires (ensuring data isn't held indefinitely)
    ///
    /// Returns a [`FutureItem`] containing the persistent (finalized) split and a reference
    /// to the temporary buffer. The persistent split is guaranteed to be time-ordered and
    /// never to be reordered by future arrivals.
    pub async fn recv(&mut self) -> Result<FutureItem<'_, Group::Output>> {
        let max_safety_lag = self.subscribers.max_safety_lag().unwrap_or(Duration::MAX);
        let mut timer = self.clock.timer(max_safety_lag);

        loop {
            let event = select! {
                _ = timer.tick() => None,
                event = self.subscribers.receive_event(&mut self.buffer) => Some(event?),
            };

            // Recalculate safe time and split the buffer
            let safe_time = self.subscribers.global_safe_time(self.clock.now());
            let mut temporary_buffer = self.buffer.split_off(&safe_time);
            std::mem::swap(&mut self.buffer, &mut temporary_buffer);
            let persistent_buffer = temporary_buffer;

            if event == Some(GroupEvent::Data) || !persistent_buffer.is_empty() {
                return Ok(FutureItem {
                    persistent: persistent_buffer,
                    temporary: &self.buffer,
                });
            }
        }
    }
}

/// Builder for constructing `FutureMap` instances.
///
/// This builder allows step-by-step addition of subscriptions to a fusion engine,
/// with each subscription providing a type, topic, and safety lag. Once all subscriptions
/// are configured, `build()` constructs the `FutureMap` ready for use.
///
/// # Example
/// ```no_run
/// let map = node
///     .create_future_map_builder()
///     .create_future_subscriber::<Imu>("sensors/imu", Duration::from_millis(5))
///     .await?
///     .create_future_subscriber::<Vision>("sensors/vision", Duration::from_millis(50))
///     .await?
///     .build();
/// ```
pub struct FutureMapBuilder<'a, Subscribers> {
    node: &'a Node,
    subscribers: Subscribers,
}

impl<'a, Subscribers> FutureMapBuilder<'a, Subscribers>
where
    Subscribers: StreamGroup,
{
    /// Constructs the `FutureMap` from the configured subscriptions.
    ///
    /// This finalizes the builder and creates a ready-to-use fusion engine
    /// with all subscriptions initialized and a clock reference for timing operations.
    pub fn build(self) -> FutureMap<Subscribers> {
        FutureMap {
            subscribers: self.subscribers,
            buffer: BTreeMap::new(),
            clock: self.node.clock().clone(),
        }
    }
}

/// Extension trait for creating future map builders on nodes.
///
/// This trait provides convenient construction of `FutureMapBuilder` instances,
/// enabling fluent configuration of multi-stream fusion.
pub trait CreateFutureMapBuilder {
    /// Create a new `FutureMapBuilder` with no subscriptions yet configured.
    ///
    /// The builder can then be extended by calling `create_future_subscriber()`
    /// repeatedly to add streams.
    fn create_future_map_builder(&self) -> FutureMapBuilder<'_, ()>;
}

impl CreateFutureMapBuilder for Node {
    fn create_future_map_builder(&self) -> FutureMapBuilder<'_, ()> {
        FutureMapBuilder {
            node: self,
            subscribers: (),
        }
    }
}

impl<'a> FutureMapBuilder<'a, ()> {
    /// Add the first subscription to this builder.
    ///
    /// Configures the initial stream subscription with the given message type,
    /// topic, and transit safety lag. Additional subscriptions can be added by
    /// chaining further calls to `create_future_subscriber()`.
    pub async fn create_future_subscriber<Type1>(
        self,
        topic: &'a str,
        safety_lag: Duration,
    ) -> Result<FutureMapBuilder<'a, (FutureQueueSubscriber<Type1>,)>>
    where
        Type1: Message,
    {
        let subscriber = self
            .node
            .create_future_subscriber(topic, safety_lag)
            .await?;
        Ok(FutureMapBuilder {
            node: self.node,
            subscribers: (subscriber,),
        })
    }
}

macro_rules! implement_stream_group {
    ($( ($type_name:ident, $index:tt) ),+) => {
        impl<$($type_name),+> StreamGroup for ($(FutureQueueSubscriber<$type_name>,)+)
        where
            $($type_name: Message + Send,)+
            $($type_name::Codec: Send + Sync,)+
        {
            type Output = ($(Option<$type_name>,)+);

            fn global_safe_time(&self, now: Time) -> Time {
                let safe_times = [ $(self.$index.safe_time(now)),+ ];
                safe_times.into_iter().min().unwrap_or(now)
            }

            fn max_safety_lag(&self) -> Option<Duration> {
                let lags = [ $(self.$index.transit_lag()),+ ];
                lags.into_iter().max()
            }

            async fn receive_event(
                &mut self,
                buffer: &mut BTreeMap<Time, Self::Output>,
    ) -> Result<GroupEvent> {
                tokio::select! {
                    $(
                        result = self.$index.recv() => {
                            // Match the QueueEvent here!
                            match result? {
                                QueueEvent::Data(time, value) => {
                                    let entry = buffer.entry(time).or_default();
                                    entry.$index = Some(value);
                                    Ok(GroupEvent::Data)
                                }
                                QueueEvent::Announcement => Ok(GroupEvent::Announcement),
                            }
                        }
                    )+
                }
            }
        }
    };
}

macro_rules! implement_builder {
    ([ $( ($type_name:ident, $index:tt) ),+ ], $next_type:ident) => {
        impl<'a, $($type_name),+> FutureMapBuilder<'a, ($(FutureQueueSubscriber<$type_name>,)+)>
        where
            $($type_name: Message,)+
        {
            /// Add another subscription to this builder.
            ///
            /// Extends the current builder with an additional stream subscription,
            /// allowing progressive construction of multi-stream fusion engines.
            pub async fn create_future_subscriber<$next_type>(
                self,
                topic: &'a str,
                safety_lag: Duration,
            ) -> Result<
                FutureMapBuilder<
                    'a,
                    ($(FutureQueueSubscriber<$type_name>,)+ FutureQueueSubscriber<$next_type>,),
                >,
            >
            where
                $next_type: Message,
            {
                let subscriber = self.node.create_future_subscriber(topic, safety_lag).await?;
                Ok(FutureMapBuilder {
                    node: self.node,
                    subscribers: ($(self.subscribers.$index,)+ subscriber,),
                })
            }
        }
    };
}

implement_stream_group!((Type1, 0));
implement_stream_group!((Type1, 0), (Type2, 1));
implement_stream_group!((Type1, 0), (Type2, 1), (Type3, 2));
implement_stream_group!((Type1, 0), (Type2, 1), (Type3, 2), (Type4, 3));
implement_stream_group!((Type1, 0), (Type2, 1), (Type3, 2), (Type4, 3), (Type5, 4));

implement_builder!([(Type1, 0)], Type2);
implement_builder!([(Type1, 0), (Type2, 1)], Type3);
implement_builder!([(Type1, 0), (Type2, 1), (Type3, 2)], Type4);
implement_builder!([(Type1, 0), (Type2, 1), (Type3, 2), (Type4, 3)], Type5);
