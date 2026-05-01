mod api;
mod engine;
mod inspect;
mod output;
mod prepare;

#[cfg(test)]
mod inspect_tests;

use std::sync::Arc;

pub use api::{
    RecorderOptions, RecordingReport, RecordingStartup, ResolvedPublisher, ResolvedTopic,
    StatsSnapshot, TopicStats,
};
use color_eyre::eyre::{Context, Result};
pub use inspect::{InspectedTopic, InspectionReport, RosZInspection, inspect_file};
pub use output::{format_output_timestamp, resolve_output_path};
pub use prepare::normalize_topics;
use ros_z::{dynamic::MessageSchema, node::Node};
use tokio::{sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;

struct TopicPlan {
    startup: ResolvedTopic,
    schema: Arc<MessageSchema>,
}

struct PreparedRecording {
    node: Arc<Node>,
    options: RecorderOptions,
    startup: RecordingStartup,
    topics: Vec<TopicPlan>,
}

pub struct RecordingPlan {
    prepared: PreparedRecording,
}

pub struct RecordingHandle {
    stats_rx: watch::Receiver<StatsSnapshot>,
    join_handle: JoinHandle<Result<RecordingReport>>,
}

impl RecordingPlan {
    pub async fn build(node: Arc<Node>, options: RecorderOptions) -> Result<Self> {
        Ok(Self {
            prepared: prepare::build(node, options).await?,
        })
    }

    pub fn startup(&self) -> &RecordingStartup {
        &self.prepared.startup
    }

    pub async fn spawn(self, shutdown: CancellationToken) -> Result<RecordingHandle> {
        engine::spawn(self.prepared, shutdown).await
    }
}

impl RecordingHandle {
    fn new(
        stats_rx: watch::Receiver<StatsSnapshot>,
        join_handle: JoinHandle<Result<RecordingReport>>,
    ) -> Self {
        Self {
            stats_rx,
            join_handle,
        }
    }

    pub fn stats(&self) -> watch::Receiver<StatsSnapshot> {
        self.stats_rx.clone()
    }

    pub async fn wait(self) -> Result<RecordingReport> {
        self.join_handle
            .await
            .context("recording supervisor task panicked")?
    }
}
