use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::debug;
use zenoh::{Session, key_expr::KeyExpr, pubsub::Subscriber, sample::SampleKind, session::ZenohId};

use crate::{
    Result,
    entity::{Entity, LivelinessKE},
    event::GraphEventManager,
};

use super::{GraphOptions, dispatch_graph_mutation, state::GraphData};

pub(super) type EntityParser = Arc<dyn Fn(&KeyExpr) -> crate::Result<Entity> + Send + Sync>;

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
    let callback_parser = parser.clone();
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

                let mutation = match sample.kind() {
                    SampleKind::Put => {
                        debug!("[GRF] Entity appeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: PUT {}", key_expr.as_str());
                        match callback_parser(&key_expr) {
                            Ok(entity) => graph_data.lock().insert(key_expr, entity),
                            Err(error) => {
                                tracing::warn!(
                                    liveliness_key = %key_expr.0,
                                    error = ?error,
                                    "failed to parse liveliness key; ignoring remote entity"
                                );
                                super::state::GraphMutation::Unchanged
                            }
                        }
                    }
                    SampleKind::Delete => {
                        debug!("[GRF] Entity disappeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: DELETE {}", key_expr.as_str());
                        graph_data.lock().remove(&key_expr)
                    }
                };

                dispatch_graph_mutation(mutation, &event_manager, &change_notify, zid);
            }
        })
        .await
        .map_err(|source| crate::Error::zenoh("declare graph liveliness subscriber", source))?;

    if let Some(timeout) = options.initial_liveliness_query_timeout {
        let replies = session
            .liveliness()
            .get(pattern)
            .timeout(timeout)
            .await
            .map_err(|source| crate::Error::zenoh("query initial graph liveliness", source))?;
        let mut reply_count = 0;
        while let Ok(reply) = replies.recv_async().await {
            reply_count += 1;
            if let Ok(sample) = reply.into_result() {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                tracing::debug!("Graph: Caching liveliness entity: {}", key_expr.as_str());
                match parser(&key_expr) {
                    Ok(entity) => {
                        graph_data.lock().insert(key_expr, entity);
                    }
                    Err(error) => {
                        tracing::warn!(
                            liveliness_key = %key_expr.0,
                            error = ?error,
                            "failed to parse initial liveliness key; ignoring remote entity"
                        );
                    }
                }
            }
        }
        tracing::debug!("Graph: Liveliness query received {} replies", reply_count);
    }

    Ok(sub)
}
