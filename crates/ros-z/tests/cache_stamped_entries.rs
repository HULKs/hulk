use std::time::Duration;

use ros_z::{context::ContextBuilder, prelude::*, time::Time};
use serde::{Deserialize, Serialize};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stamped_entries_cache_expands_one_message_into_many_entries() -> Result {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("stamped_entries_pub").build().await?;
    let cache_node = context.create_node("stamped_entries_cache").build().await?;
    let topic = "/cache_stamped_entries_many";

    let cache = cache_node
        .create_cache::<Batch>(topic, 10)?
        .with_stamped_entries(|batch| {
            batch
                .entries
                .into_iter()
                .map(|entry| (entry.time, entry.value))
        })
        .build()
        .await?;

    let publisher = publisher_node.publisher::<Batch>(topic)?.build().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

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

    tokio::time::sleep(Duration::from_millis(200)).await;

    let ten = cache.get_nearest(Time::from_nanos(10)).unwrap();
    let twenty = cache.get_nearest(Time::from_nanos(20)).unwrap();

    assert_eq!(ten.as_ref(), "ten");
    assert_eq!(twenty.as_ref(), "twenty");
    assert_eq!(cache.len(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stamped_entries_cache_accepts_empty_iterators_and_respects_capacity() -> Result {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("stamped_entries_capacity_pub")
        .build()
        .await?;
    let cache_node = context
        .create_node("stamped_entries_capacity_cache")
        .build()
        .await?;
    let topic = "/cache_stamped_entries_capacity";

    let cache = cache_node
        .create_cache::<Batch>(topic, 2)?
        .with_stamped_entries(|batch| {
            batch
                .entries
                .into_iter()
                .map(|entry| (entry.time, entry.value))
        })
        .build()
        .await?;

    let publisher = publisher_node.publisher::<Batch>(topic)?.build().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    publisher.publish(&Batch { entries: vec![] }).await?;
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

    tokio::time::sleep(Duration::from_millis(200)).await;

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
