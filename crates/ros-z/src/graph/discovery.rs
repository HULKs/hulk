use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Notify, watch};
use tracing::{debug, warn};
use zenoh::{Session, pubsub::Subscriber, sample::SampleKind};

use crate::{Result, entity::LivelinessKE};

use super::{GraphOptions, state::GraphData};
use ros_z_protocol::format::parse_liveliness;

pub(super) async fn install_liveliness(
    session: &Session,
    pattern: &str,
    options: &GraphOptions,
    graph_data: Arc<Mutex<GraphData>>,
    change_notify: Arc<Notify>,
    change_revision: watch::Sender<u64>,
) -> Result<Subscriber<()>> {
    debug!(pattern = %pattern, "declaring graph liveliness subscriber");
    let initial_hydration_complete = Arc::new(AtomicBool::new(false));
    let sub = session
        .liveliness()
        .declare_subscriber(pattern)
        .callback({
            let graph_data = graph_data.clone();
            let initial_hydration_complete = initial_hydration_complete.clone();
            move |sample| {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                let sample_kind = sample.kind();
                debug!(
                    liveliness_key = %key_expr.0,
                    kind = ?sample_kind,
                    "received graph liveliness token"
                );

                let changed = match sample_kind {
                    SampleKind::Put => match parse_liveliness(&key_expr) {
                        Ok(entity) => graph_data.lock().insert(key_expr, entity),
                        Err(error) => {
                            warn!(
                                liveliness_key = %key_expr.0,
                                error = ?error,
                                "failed to parse liveliness key; ignoring remote entity"
                            );
                            false
                        }
                    },
                    SampleKind::Delete => graph_data.lock().remove(&key_expr),
                };

                if changed && initial_hydration_complete.load(Ordering::Acquire) {
                    change_revision.send_modify(|revision| *revision = revision.saturating_add(1));
                    change_notify.notify_waiters();
                }
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
                debug!(
                    liveliness_key = %key_expr.0,
                    "received initial graph liveliness token"
                );
                match parse_liveliness(&key_expr) {
                    Ok(entity) => {
                        let _ = graph_data.lock().insert(key_expr, entity);
                    }
                    Err(error) => {
                        warn!(
                            liveliness_key = %key_expr.0,
                            error = ?error,
                            "failed to parse initial liveliness key; ignoring remote entity"
                        );
                    }
                }
            }
        }
        debug!(reply_count, "initial graph liveliness query completed");
    }
    initial_hydration_complete.store(true, Ordering::Release);

    Ok(sub)
}
