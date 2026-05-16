use std::collections::BTreeMap;

use ros_z::{Message, Result, node::Node, time::Time};

use crate::future_queue::{CreateFutureQueue, FutureQueueSubscriber};

pub type FutureResult<Types> = BTreeMap<Time, Types>;

#[derive(Debug)]
pub struct FutureItem<'a, Types> {
    pub persistent: FutureResult<Types>,
    pub temporary: &'a FutureResult<Types>,
}

pub trait StreamGroup {
    type Output: Default;

    fn global_safe_time(&self) -> Option<Time>;

    // Required to abstract the varying arity of the streams for tokio::select!
    fn receive_into(
        &mut self,
        buffer: &mut BTreeMap<Time, Self::Output>,
    ) -> impl Future<Output = Result<()>>;
}

pub struct FutureMap<Group: StreamGroup> {
    subscribers: Group,
    buffer: FutureResult<Group::Output>,
}

impl<Group: StreamGroup> FutureMap<Group> {
    pub async fn recv(&mut self) -> Result<FutureItem<'_, Group::Output>> {
        self.subscribers.receive_into(&mut self.buffer).await?;

        let mut persistent_buffer = BTreeMap::new();
        if let Some(safe_time) = self.subscribers.global_safe_time() {
            let temporary_buffer = self.buffer.split_off(&safe_time);
            std::mem::swap(&mut persistent_buffer, &mut self.buffer);
            self.buffer = temporary_buffer;
        } else {
            std::mem::swap(&mut persistent_buffer, &mut self.buffer);
        }

        Ok(FutureItem {
            persistent: persistent_buffer,
            temporary: &self.buffer,
        })
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
    ) -> Result<FutureMapBuilder<'a, (FutureQueueSubscriber<Type1>,)>>
    where
        Type1: Message,
    {
        let subscriber = self.node.create_future_subscriber(topic).await?;
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

            fn global_safe_time(&self) -> Option<Time> {
                let safe_times = [ $(self.$index.safe_time()),+ ];
                safe_times.into_iter().flatten().min()
            }

            async fn receive_into(
                &mut self,
                buffer: &mut BTreeMap<Time, Self::Output>,
            ) -> Result<()> {
                tokio::select! {
                    $(
                        result = self.$index.recv() => {
                            let (time, value) = result?;
                            let entry = buffer.entry(time).or_default();
                            entry.$index = Some(value);
                            Ok(())
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
            ) -> Result<
                FutureMapBuilder<
                    'a,
                    ($(FutureQueueSubscriber<$type_name>,)+ FutureQueueSubscriber<$next_type>,),
                >,
            >
            where
                $next_type: Message,
            {
                let subscriber = self.node.create_future_subscriber(topic).await?;
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
