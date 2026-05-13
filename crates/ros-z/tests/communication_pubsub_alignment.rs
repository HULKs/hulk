use std::time::Duration;

use ros_z::context::ContextBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::AlignmentMessage")]
struct AlignmentMessage {
    value: u64,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publisher_message_id_is_reserved_and_observable() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("alignment_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<AlignmentMessage>("/alignment_topic")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create publisher");
    let subscriber = node
        .subscriber::<AlignmentMessage>("/alignment_topic")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create subscriber");

    assert!(
        publisher
            .wait_for_subscribers(1, Duration::from_secs(2))
            .await,
        "subscriber did not appear in time"
    );

    let message = AlignmentMessage { value: 42 };
    let publish = publisher.prepare();
    let publication_id = publish.id();

    publish
        .publish(&message)
        .await
        .expect("Failed to publish message");

    let received = subscriber
        .recv_with_metadata()
        .await
        .expect("Failed to receive message");

    assert_eq!(received.message(), &message);
    assert_eq!(received.publication_id(), Some(publication_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publisher_direct_publish_attaches_publication_id() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("alignment_direct_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<AlignmentMessage>("/alignment_direct_topic")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create publisher");
    let subscriber = node
        .subscriber::<AlignmentMessage>("/alignment_direct_topic")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create subscriber");

    assert!(
        publisher
            .wait_for_subscribers(1, Duration::from_secs(2))
            .await,
        "subscriber did not appear in time"
    );

    let message = AlignmentMessage { value: 7 };

    publisher
        .publish(&message)
        .await
        .expect("Failed to publish message");

    let received = subscriber
        .recv_with_metadata()
        .await
        .expect("Failed to receive message");

    assert_eq!(received.message(), &message);
    assert!(
        received.publication_id().is_some(),
        "direct publish should still carry a publication id"
    );
}
