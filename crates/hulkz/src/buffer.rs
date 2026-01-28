use std::{future::Future, sync::Arc};

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{subscriber::SubscriberError, Cache, Message, Subscriber, Timestamp};

#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Subscriber error: {0}")]
    SubscriberError(#[from] SubscriberError),
}

pub type Result<T, E = BufferError> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct Buffer<T> {
    cache: Arc<RwLock<Cache<Arc<Message<T>>>>>,
}

impl<T> Buffer<T>
where
    for<'de> T: Deserialize<'de> + Clone + Send + 'static,
{
    pub fn new(
        mut subscriber: Subscriber<T>,
        capacity: usize,
    ) -> (Self, impl Future<Output = Result<()>>) {
        let cache = Arc::new(RwLock::new(Cache::new(capacity)));

        let driver = {
            let cache = cache.clone();
            async move {
                loop {
                    let message = subscriber.recv_async().await?;
                    let mut cache = cache.write().await;
                    cache.add(message.timestamp, Arc::new(message));
                }
            }
        };

        let handle = Self { cache };

        (handle, driver)
    }

    pub async fn lookup_nearest(&self, timestamp: &Timestamp) -> Option<Arc<Message<T>>> {
        let cache = self.cache.read().await;

        let before = cache.get_elem_before_time(timestamp);
        let after = cache.get_elem_after_time(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => {
                let diff_to_before = timestamp
                    .get_time()
                    .to_duration()
                    .abs_diff(before.timestamp.get_time().to_duration());
                let diff_to_after = timestamp
                    .get_time()
                    .to_duration()
                    .abs_diff(after.timestamp.get_time().to_duration());
                if diff_to_before < diff_to_after {
                    Some(before.clone())
                } else {
                    Some(after.clone())
                }
            }
            (Some(x), None) => Some(x.clone()),
            (None, Some(x)) => Some(x.clone()),
            (None, None) => None,
        }
    }

    pub async fn get_latest(&self) -> Option<Arc<Message<T>>> {
        self.cache.read().await.get_latest().cloned()
    }
}
