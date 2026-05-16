use std::collections::BTreeMap;
use std::time::Duration;

use ros_z::Message;
use ros_z::Result as ZResult;
use ros_z::node::Node;
use ros_z::time::Clock;
use ros_z::time::Time;
use tokio::select;

use crate::future_queue::{CreateFutureQueue, FutureQueueSubscriber, QueueEvent};

pub type FutureResult<Types> = BTreeMap<Time, Types>;

#[derive(Debug)]
pub struct FutureItem<'a, Types> {
    pub persistent: FutureResult<Types>,
    pub temporary: &'a FutureResult<Types>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GroupEvent {
    Announcement,
    Data,
}

pub trait StreamGroup {
    type Output: Default;

    fn global_safe_time(&self, now: Time) -> Time;

    fn max_safety_lag(&self) -> Option<Duration>;

    // Required to abstract the varying arity of the streams for tokio::select!
    fn receive_event(
        &mut self,
        buffer: &mut BTreeMap<Time, Self::Output>,
    ) -> impl Future<Output = ZResult<GroupEvent>>;
}

#[derive(Debug)]
pub struct FutureMap<Group: StreamGroup> {
    subscribers: Group,
    buffer: FutureResult<Group::Output>,
    clock: Clock,
}

impl<Group: StreamGroup> FutureMap<Group> {
    pub async fn recv(&mut self) -> ZResult<FutureItem<'_, Group::Output>> {
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

pub struct FutureMapBuilder<'a, Subscribers> {
    node: &'a Node,
    subscribers: Subscribers,
}

impl<'a, Subscribers> FutureMapBuilder<'a, Subscribers>
where
    Subscribers: StreamGroup,
{
    pub fn build(self) -> FutureMap<Subscribers> {
        FutureMap {
            subscribers: self.subscribers,
            buffer: BTreeMap::new(),
            clock: self.node.clock().clone(),
        }
    }
}

pub trait CreateFutureMapBuilder {
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
    pub async fn create_future_subscriber<Type1>(
        self,
        topic: &'a str,
        safety_lag: Duration,
    ) -> ZResult<FutureMapBuilder<'a, (FutureQueueSubscriber<Type1>,)>>
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
                let lags = [ $(self.$index.safety_lag()),+ ];
                lags.into_iter().max()
            }

            async fn receive_event(
                &mut self,
                buffer: &mut BTreeMap<Time, Self::Output>,
            ) -> ZResult<GroupEvent> {
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
            pub async fn create_future_subscriber<$next_type>(
                self,
                topic: &'a str,
                safety_lag: Duration,
            ) -> ZResult<
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
