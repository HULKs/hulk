use std::{
    future::Future,
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::Deserialize;

use crate::{Cache, Timestamped, TopicStream};

#[derive(Clone)]
pub struct TopicBuffer<T> {
    cache: Arc<RwLock<Cache<T>>>,
}

impl<T> TopicBuffer<T>
where
    for<'de> T: Deserialize<'de> + Timestamped + Clone + Send + 'static,
{
    pub fn new(mut stream: TopicStream<T>, capacity: usize) -> (Self, impl Future<Output = ()>) {
        let cache = Arc::new(RwLock::new(Cache::new(capacity)));

        let driver = {
            let cache = cache.clone();
            async move {
                while let Ok(msg) = stream.recv_async().await {
                    let mut cache = cache.write().expect("lock is poisoned");
                    cache.add(msg);
                }
            }
        };

        let handle = Self { cache };

        (handle, driver)
    }

    pub fn lookup_nearest(&self, timestamp: Duration) -> Option<T> {
        let cache = self.cache.read().expect("lock is poisoned");

        let before = cache.get_elem_before_time(timestamp);
        let after = cache.get_elem_after_time(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => {
                let diff_to_before = timestamp.abs_diff(before.timestamp());
                let diff_to_after = timestamp.abs_diff(after.timestamp());
                if diff_to_before < diff_to_after {
                    Some(before)
                } else {
                    Some(after)
                }
            }
            (Some(x), None) => Some(x),
            (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }

    pub fn get_latest(&self) -> Option<T> {
        self.cache.read().expect("lock is poisoned").get_latest()
    }
}
