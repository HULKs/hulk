use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::debug;
use zenoh::{Result, Session, pubsub::Subscriber, sample::SampleKind, session::ZenohId};

use crate::{
    entity::{Entity, LivelinessKE},
    event::GraphEventManager,
};

use super::{
    GraphOptions,
    state::{EntityParser, GraphData},
};

#[expect(
    clippy::too_many_arguments,
    reason = "arguments mirror the graph components installed by this private discovery seam"
)]
pub(super) async fn install_liveliness(
    session: &Session,
    pattern: &str,
    parser: EntityParser,
    options: &GraphOptions,
    graph_data: Arc<Mutex<GraphData>>,
    event_manager: Arc<GraphEventManager>,
    change_notify: Arc<Notify>,
    zid: ZenohId,
) -> Result<Subscriber<()>> {
    let callback_parser = parser;
    tracing::debug!("Creating liveliness subscriber for {}", pattern);
    let sub = session
        .liveliness()
        .declare_subscriber(pattern)
        .history(true)
        .callback({
            let graph_data = graph_data.clone();
            move |sample| {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                tracing::debug!(
                    "Received liveliness token: {} kind={:?}",
                    key_expr.0,
                    sample.kind()
                );

                let graph_change: Option<(Entity, bool)> = match sample.kind() {
                    SampleKind::Put => {
                        debug!("[GRF] Entity appeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: PUT {}", key_expr.as_str());
                        let parsed_entity = match callback_parser(&key_expr) {
                            Ok(entity) => Some(entity),
                            Err(error) => {
                                tracing::warn!(
                                    "Failed to parse liveliness token {}: {:?}",
                                    key_expr.0,
                                    error
                                );
                                None
                            }
                        };
                        graph_data.lock().insert(key_expr);
                        parsed_entity.map(|entity| (entity, true))
                    }
                    SampleKind::Delete => {
                        debug!("[GRF] Entity disappeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: DELETE {}", key_expr.as_str());
                        let parsed_entity = callback_parser(&key_expr).ok();
                        graph_data.lock().remove(&key_expr);
                        parsed_entity.map(|entity| (entity, false))
                    }
                };

                if let Some((entity, appeared)) = graph_change {
                    event_manager.trigger_graph_change(&entity, appeared, zid);
                }
                change_notify.notify_waiters();
            }
        })
        .await?;

    if let Some(timeout) = options.initial_liveliness_query_timeout {
        let replies = session.liveliness().get(pattern).timeout(timeout).await?;
        let mut reply_count = 0;
        while let Ok(reply) = replies.recv_async().await {
            reply_count += 1;
            if let Ok(sample) = reply.into_result() {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                tracing::debug!("Graph: Caching liveliness entity: {}", key_expr.as_str());
                graph_data.lock().insert(key_expr);
            }
        }
        tracing::debug!("Graph: Liveliness query received {} replies", reply_count);
    }

    Ok(sub)
}
