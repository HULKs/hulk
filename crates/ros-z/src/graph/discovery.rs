use std::sync::Arc;

use tracing::{debug, warn};
use zenoh::{Session, pubsub::Subscriber, sample::Sample, sample::SampleKind};

use crate::{Result, entity::LivelinessKE};

use super::state::GraphInner;
use ros_z_protocol::format::parse_liveliness;

pub(super) async fn install_liveliness(
    session: &Session,
    pattern: &str,
    graph: Arc<GraphInner>,
) -> Result<Subscriber<()>> {
    debug!(pattern = %pattern, "declaring graph liveliness subscriber");
    let sub = session
        .liveliness()
        .declare_subscriber(pattern)
        .history(true)
        .callback(move |sample| {
            if let Err(error) = handle_liveliness_sample(&graph, sample) {
                warn!(%error, "failed to handle ros-z graph liveliness sample");
            }
        })
        .await
        .map_err(|source| crate::Error::zenoh("declare graph liveliness subscriber", source))?;

    Ok(sub)
}

fn handle_liveliness_sample(graph: &GraphInner, sample: Sample) -> Result<()> {
    let key_expr = LivelinessKE(sample.key_expr().to_owned());
    debug!(
        liveliness_key = %key_expr.0,
        kind = ?sample.kind(),
        "received graph liveliness token"
    );
    match sample.kind() {
        SampleKind::Put => {
            let entity = match parse_liveliness(&key_expr) {
                Ok(entity) => entity,
                Err(error) => {
                    warn!(
                        liveliness_key = %key_expr.0,
                        error = ?error,
                        "failed to parse liveliness key; ignoring remote entity"
                    );
                    return Ok(());
                }
            };
            graph.insert(key_expr, entity);
        }
        SampleKind::Delete => {
            graph.remove(&key_expr);
        }
    }
    Ok(())
}
