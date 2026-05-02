use std::collections::BTreeMap;

use ros_z::Message;
use ros_z::Result as ZResult;
use ros_z::msg::WireDecoder;
use ros_z::node::Node;
use ros_z::time::Time;

use crate::future_queue::{
    CreateFutureQueue, FutureQueueSubscriber, LagPolicy, LagWarning, QueueEvent, QueueState,
};

/// Time-ordered fusion buffer keyed by message timestamp.
pub type FutureResult<Types> = BTreeMap<Time, Types>;

/// One fusion step output split into persistent and temporary windows.
pub struct FutureItem<'a, Types> {
    /// Entries guaranteed never to be invalidated by future arrivals.
    pub persistent: FutureResult<Types>,
    /// Entries still provisional because some stream may deliver older data.
    pub temporary: &'a FutureResult<Types>,
}

/// Fusion output plus per-stream queue states and warnings.
pub struct FutureReceive<'a, DataTypes, StateTypes> {
    /// Fused data split into persistent and temporary windows.
    pub item: FutureItem<'a, DataTypes>,
    /// Per-stream state tuple aligned with stream ordering.
    pub stream_states: StateTypes,
}

/// Type-level description for tuple-based stream groups.
pub trait StreamGroup {
    /// Tuple of concrete queue subscribers.
    type Subscribers;
    /// Tuple of fused payload slots (`Option<T>` per stream).
    type Output;
}

macro_rules! output_tuple {
    ($($type_name:ident),+) => {
        ($(Option<$type_name>,)+)
    };
}

macro_rules! replace_type {
    ($_src:ty, $dst:ty) => {
        $dst
    };
}

macro_rules! state_tuple {
    ($($type_name:ident),+) => {
        ($(replace_type!($type_name, QueueState),)+)
    };
}

/// Multi-stream fusion engine that maintains persistent/temporary boundaries.
pub struct FutureMap<Group: StreamGroup> {
    subscribers: Group::Subscribers,
    buffer: FutureResult<Group::Output>,
}

impl<Group: StreamGroup> FutureMap<Group> {
    /// Construct map with subscribers and empty fusion buffer.
    fn new_with(subscribers: Group::Subscribers) -> Self {
        Self {
            subscribers,
            buffer: BTreeMap::new(),
        }
    }

    /// Latch first warning for stream until current receive returns.
    fn latch_warning(
        latched_warnings: &mut [Option<LagWarning>],
        index: usize,
        warning: Option<LagWarning>,
    ) {
        if latched_warnings[index].is_none() {
            latched_warnings[index] = warning;
        }
    }

    /// Apply queue state to receive-local stream state arrays.
    fn apply_state(
        safe_times: &mut [Option<Time>],
        latched_warnings: &mut [Option<LagWarning>],
        index: usize,
        state: QueueState,
    ) {
        safe_times[index] = state.safe_time;
        Self::latch_warning(latched_warnings, index, state.warning);
    }

    /// Finalize receive by splitting buffer at computed safe time.
    fn finalize_receive<StateTypes>(
        &mut self,
        stream_states: StateTypes,
        global_safe_time: Option<Time>,
    ) -> FutureReceive<'_, Group::Output, StateTypes> {
        let item = self.split_buffer(global_safe_time);
        FutureReceive {
            item,
            stream_states,
        }
    }

    fn global_safe_time(safe_times: &[Option<Time>]) -> Option<Time> {
        let mut global_safe_time = None;
        for inflight_message in safe_times {
            match (global_safe_time, inflight_message) {
                (None, Some(time)) => global_safe_time = Some(*time),
                (Some(safe), Some(time)) if time < &safe => global_safe_time = Some(*time),
                _ => {}
            }
        }
        global_safe_time
    }

    fn split_buffer(&mut self, global_safe_time: Option<Time>) -> FutureItem<'_, Group::Output> {
        let mut persistent_buffer = BTreeMap::new();
        if let Some(safe_time) = global_safe_time {
            let temporary_buffer = self.buffer.split_off(&safe_time);
            std::mem::swap(&mut persistent_buffer, &mut self.buffer);
            self.buffer = temporary_buffer;
        } else {
            std::mem::swap(&mut persistent_buffer, &mut self.buffer);
        }

        FutureItem {
            persistent: persistent_buffer,
            temporary: &self.buffer,
        }
    }

    fn has_releasable_buffer(&self, global_safe_time: Option<Time>) -> bool {
        match global_safe_time {
            Some(safe_time) => self
                .buffer
                .keys()
                .next()
                .is_some_and(|time| time < &safe_time),
            None => !self.buffer.is_empty(),
        }
    }
}

/// Builder for tuple-based [`FutureMap`] instances.
pub struct FutureMapBuilder<'a, Group: StreamGroup> {
    node: &'a Node,
    subscribers: Group::Subscribers,
}

/// Extension trait for creating a [`FutureMapBuilder`].
pub trait CreateFutureMapBuilder {
    /// Start building a future map with zero streams.
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

impl StreamGroup for () {
    type Subscribers = ();
    type Output = ();
}

impl<'a> FutureMapBuilder<'a, ()> {
    /// Add first stream subscriber to the map.
    pub async fn create_future_subscriber<T>(
        self,
        topic: &'a str,
        lag_policy: LagPolicy,
    ) -> ZResult<FutureMapBuilder<'a, (T,)>>
    where
        T: Message,
        for<'de> T::Codec: WireDecoder<Input<'de> = &'de [u8], Output = T>,
    {
        let subscriber = self
            .node
            .create_future_subscriber(topic, lag_policy)
            .await?;
        Ok(FutureMapBuilder {
            node: self.node,
            subscribers: (subscriber,),
        })
    }
}

macro_rules! implement_stream_group {
    (@core $length:expr, [ $( ($type_name:ident, $index:tt) ),+ ]) => {
        impl<$($type_name),+> StreamGroup for ($($type_name,)+)
        where
            $($type_name: Message,)+
            $(for<'de> <$type_name as Message>::Codec: WireDecoder<Input<'de> = &'de [u8], Output = $type_name>,)+
        {
            type Subscribers = ($(FutureQueueSubscriber<$type_name>,)+);
            type Output = output_tuple!($($type_name),+);
        }

        impl<'a, $($type_name),+> FutureMapBuilder<'a, ($($type_name,)+)>
        where
            $($type_name: Message,)+
            $(for<'de> <$type_name as Message>::Codec: WireDecoder<Input<'de> = &'de [u8], Output = $type_name>,)+
        {
            /// Finalize builder into a ready-to-receive future map.
            pub fn build(self) -> FutureMap<($($type_name,)+)> {
                FutureMap::new_with(self.subscribers)
            }
        }

        impl<$($type_name),+> FutureMap<($($type_name,)+)>
        where
            output_tuple!($($type_name),+): Default,
            $($type_name: Message,)+
            $(for<'de> <$type_name as Message>::Codec: WireDecoder<Input<'de> = &'de [u8], Output = $type_name>,)+
        {
            fn snapshot_states(&mut self, latched_warnings: &mut [Option<LagWarning>; $length]) -> state_tuple!($($type_name),+) {
                (
                    $(
                        {
                            let mut s = self.subscribers.$index.current_state();
                            s.warning = s.warning.or(latched_warnings[$index].take());
                            s
                        },
                    )+
                )
            }

            /// Wait until data arrives on any stream and return one fusion step.
            ///
            /// Announcement-only events are consumed internally to update safe-time.
            /// This method returns only when a data payload is incorporated.
            pub async fn recv(&mut self) -> ZResult<FutureReceive<'_, output_tuple!($($type_name),+), state_tuple!($($type_name),+)>> {
                let mut safe_times = [None; $length];
                let mut latched_warnings = [None; $length];

                $(
                    let state = self.subscribers.$index.drain_announcements().await?;
                    Self::apply_state(&mut safe_times, &mut latched_warnings, $index, state);
                )+

                let global_safe_time = Self::global_safe_time(&safe_times);
                if self.has_releasable_buffer(global_safe_time) {
                    let stream_states = self.snapshot_states(&mut latched_warnings);
                    return Ok(self.finalize_receive(stream_states, global_safe_time));
                }

                loop {
                    tokio::select! {
                        $(
                            result = self.subscribers.$index.recv() => {
                                match result? {
                                    QueueEvent::Announcement { state } => {
                                        Self::apply_state(&mut safe_times, &mut latched_warnings, $index, state);
                                        let global_safe_time = Self::global_safe_time(&safe_times);
                                        if self.has_releasable_buffer(global_safe_time) {
                                            let stream_states = self.snapshot_states(&mut latched_warnings);
                                            return Ok(self.finalize_receive(stream_states, global_safe_time));
                                        }
                                    }
                                    QueueEvent::Data { state, data_time, value } => {
                                        Self::apply_state(&mut safe_times, &mut latched_warnings, $index, state);

                                        let entry = self.buffer.entry(data_time).or_insert_with(Default::default);
                                        entry.$index = Some(value);

                                        let stream_states = self.snapshot_states(&mut latched_warnings);
                                        let global_safe_time = Self::global_safe_time(&safe_times);
                                        return Ok(self.finalize_receive(stream_states, global_safe_time));
                                    }
                                }
                            }
                        )+
                    }
                }
            }
        }
    };

    (@next [ $( ($type_name:ident, $index:tt) ),+ ], $next_type:ident) => {
        impl<'a, $($type_name),+> FutureMapBuilder<'a, ($($type_name,)+)>
        where
            $($type_name: Message,)+
            $(for<'de> <$type_name as Message>::Codec: WireDecoder<Input<'de> = &'de [u8], Output = $type_name>,)+
        {
            /// Append another stream subscriber to this map builder.
            pub async fn create_future_subscriber<$next_type>(
                self,
                topic: &'a str,
                lag_policy: LagPolicy,
            ) -> ZResult<FutureMapBuilder<'a, ($($type_name,)+ $next_type,)>>
            where
                $next_type: Message,
                for<'de> <$next_type as Message>::Codec: WireDecoder<Input<'de> = &'de [u8], Output = $next_type>,
            {
                let new_subscriber = self.node.create_future_subscriber(topic, lag_policy).await?;
                Ok(FutureMapBuilder {
                    node: self.node,
                    subscribers: ($(self.subscribers.$index,)+ new_subscriber,),
                })
            }
        }
    };

    ($length:expr, [ $( ($type_name:ident, $index:tt) ),+ ], next: $next_type:ident) => {
        implement_stream_group!(@core $length, [ $( ($type_name, $index) ),+ ]);
        implement_stream_group!(@next [ $( ($type_name, $index) ),+ ], $next_type);
    };

    ($length:expr, [ $( ($type_name:ident, $index:tt) ),+ ]) => {
        implement_stream_group!(@core $length, [ $( ($type_name, $index) ),+ ]);
    };
}

implement_stream_group!(1, [(T1, 0)], next: T2);
implement_stream_group!(2, [(T1, 0), (T2, 1)], next: T3);
implement_stream_group!(3, [(T1, 0), (T2, 1), (T3, 2)], next: T4);
implement_stream_group!(4, [(T1, 0), (T2, 1), (T3, 2), (T4, 3)], next: T5);
implement_stream_group!(5, [(T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4)], next: T6);
implement_stream_group!(6, [(T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5)], next: T7);
implement_stream_group!(7, [(T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6)], next: T8);
implement_stream_group!(
    8,
    [
        (T1, 0),
        (T2, 1),
        (T3, 2),
        (T4, 3),
        (T5, 4),
        (T6, 5),
        (T7, 6),
        (T8, 7)
    ]
);
