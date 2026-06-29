use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::eyre::{Result, WrapErr};
use ros_z::{
    context::{Context, ContextBuilder},
    dynamic::{DynamicRawSubscriberDiscoveryBuilder, DynamicSubscriber},
    graph::GraphSnapshot,
    node::Node,
    parameter::RemoteParameterClient,
};

use crate::support::graph::SnapshotFingerprint;

const GRAPH_POLL_INTERVAL: Duration = Duration::from_millis(200);
const GRAPH_SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct AppContext {
    context: Context,
    node: Arc<Node>,
}

impl AppContext {
    pub async fn new(router: &str) -> Result<Self> {
        let context = ContextBuilder::default()
            .with_mode("client")
            .with_connect_endpoints([router])
            .build()
            .await
            .wrap_err("failed to build ros-z context")?;
        let node = Arc::new(
            context
                .create_node("rosz")
                .build()
                .await
                .wrap_err("failed to build rosz node")?,
        );

        Ok(Self { context, node })
    }

    pub fn graph(&self) -> &ros_z::graph::Graph {
        self.context.graph().as_ref()
    }

    pub fn node(&self) -> Arc<Node> {
        Arc::clone(&self.node)
    }

    pub fn snapshot(&self) -> GraphSnapshot {
        self.graph().snapshot()
    }

    pub async fn wait_for_graph_settle(&self) {
        let deadline = Instant::now() + GRAPH_SETTLE_TIMEOUT;
        let mut previous = SnapshotFingerprint::from(&self.snapshot());

        while Instant::now() < deadline {
            tokio::time::sleep(GRAPH_POLL_INTERVAL).await;

            let current_snapshot = self.snapshot();
            let current = SnapshotFingerprint::from(&current_snapshot);
            if current == previous {
                return;
            }
            previous = current;
        }
    }

    pub async fn wait_for_graph_condition<F>(&self, predicate: F)
    where
        F: Fn(&ros_z::graph::Graph) -> bool,
    {
        let deadline = Instant::now() + GRAPH_SETTLE_TIMEOUT;

        while Instant::now() < deadline {
            if predicate(self.graph()) {
                return;
            }
            tokio::time::sleep(GRAPH_POLL_INTERVAL).await;
        }
    }

    pub async fn create_dynamic_subscriber(
        &self,
        topic: &str,
        discovery_timeout: Duration,
    ) -> Result<DynamicSubscriber> {
        self.node
            .dynamic_subscriber_auto(topic, discovery_timeout)
            .build()
            .await
            .wrap_err_with(|| format!("failed to subscribe to {topic}"))
    }

    pub fn create_raw_subscriber_builder(
        &self,
        topic: &str,
        discovery_timeout: Duration,
    ) -> DynamicRawSubscriberDiscoveryBuilder {
        self.node
            .dynamic_subscriber_auto(topic, discovery_timeout)
            .raw()
    }

    pub fn parameter_client(&self, target_fqn: &str) -> Result<RemoteParameterClient> {
        let client = RemoteParameterClient::new(Arc::clone(&self.node), target_fqn)?;
        Ok(client)
    }

    pub fn shutdown(&self) -> Result<()> {
        self.context
            .shutdown()
            .wrap_err("failed to close ros-z context")
    }
}
