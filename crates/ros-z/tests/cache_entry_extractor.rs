use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use ros_z::{context::ContextBuilder, entity::EntityKind, prelude::*, time::Time};
use serde::{Deserialize, Serialize};

const EVENTUALLY_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
struct Batch {
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
struct Entry {
    time: Time,
    value: String,
}

async fn eventually(description: &str, mut condition: impl FnMut() -> bool) {
    let start = Instant::now();
    loop {
        if condition() {
            return;
        }

        assert!(
            start.elapsed() < EVENTUALLY_TIMEOUT,
            "{description} did not happen within {EVENTUALLY_TIMEOUT:?}"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn wait_for_subscriber(node: &ros_z::node::Node, topic: &str) {
    eventually("cache subscriber discovery", || {
        node.graph().count(EntityKind::Subscription, topic) >= 1
    })
    .await;
}

async fn wait_for_cache_len<T>(cache: &ros_z::cache::Cache<T>, expected_len: usize) {
    eventually("cache length update", || cache.len() == expected_len).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn entry_extractor_cache_expands_one_message_into_many_entries() -> Result {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("entry_extractor_pub").build().await?;
    let cache_node = context.create_node("entry_extractor_cache").build().await?;
    let topic = "/cache_entry_extractor_many";

    let cache = cache_node
        .create_cache::<Batch>(topic, 10)?
        .with_entry_extractor(|batch| {
            batch
                .entries
                .into_iter()
                .map(|entry| (entry.time, entry.value))
        })
        .build()
        .await?;

    let publisher = publisher_node.publisher::<Batch>(topic)?.build().await?;
    wait_for_subscriber(&publisher_node, topic).await;

    publisher
        .publish(&Batch {
            entries: vec![
                Entry {
                    time: Time::from_nanos(10),
                    value: "ten".to_string(),
                },
                Entry {
                    time: Time::from_nanos(20),
                    value: "twenty".to_string(),
                },
            ],
        })
        .await?;

    wait_for_cache_len(&cache, 2).await;

    let ten = cache.get_nearest(Time::from_nanos(10)).unwrap();
    let twenty = cache.get_nearest(Time::from_nanos(20)).unwrap();

    assert_eq!(ten.as_ref(), "ten");
    assert_eq!(twenty.as_ref(), "twenty");
    assert_eq!(cache.len(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn entry_extractor_cache_accepts_empty_iterators_and_respects_capacity() -> Result {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("entry_extractor_capacity_pub")
        .build()
        .await?;
    let cache_node = context
        .create_node("entry_extractor_capacity_cache")
        .build()
        .await?;
    let topic = "/cache_entry_extractor_capacity";
    let extracted_batches = Arc::new(AtomicUsize::new(0));
    let extracted_batches_cb = Arc::clone(&extracted_batches);

    let cache = cache_node
        .create_cache::<Batch>(topic, 2)?
        .with_entry_extractor(move |batch| {
            extracted_batches_cb.fetch_add(1, Ordering::SeqCst);
            batch
                .entries
                .into_iter()
                .map(|entry| (entry.time, entry.value))
        })
        .build()
        .await?;

    let publisher = publisher_node.publisher::<Batch>(topic)?.build().await?;
    wait_for_subscriber(&publisher_node, topic).await;

    publisher.publish(&Batch { entries: vec![] }).await?;
    eventually("empty batch extraction", || {
        extracted_batches.load(Ordering::SeqCst) >= 1
    })
    .await;

    publisher
        .publish(&Batch {
            entries: vec![
                Entry {
                    time: Time::from_nanos(10),
                    value: "dropped".to_string(),
                },
                Entry {
                    time: Time::from_nanos(20),
                    value: "kept-a".to_string(),
                },
                Entry {
                    time: Time::from_nanos(30),
                    value: "kept-b".to_string(),
                },
            ],
        })
        .await?;

    wait_for_cache_len(&cache, 2).await;
    eventually("all batches extracted", || {
        extracted_batches.load(Ordering::SeqCst) >= 2
    })
    .await;

    assert_eq!(cache.len(), 2);
    assert_eq!(
        cache.get_nearest(Time::from_nanos(20)).unwrap().as_ref(),
        "kept-a"
    );
    assert_eq!(
        cache.get_nearest(Time::from_nanos(30)).unwrap().as_ref(),
        "kept-b"
    );
    assert_eq!(cache.earliest_stamp(), Some(Time::from_nanos(20)));

    Ok(())
}
