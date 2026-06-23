use tracing::{debug, warn};
use zenoh::{Session, pubsub::Subscriber, sample::SampleKind};

use crate::{Result, entity::LivelinessKE};

use super::state::GraphStore;
use ros_z_protocol::format::parse_liveliness;

pub(super) async fn install_liveliness(
    session: &Session,
    pattern: &str,
    graph_store: GraphStore,
) -> Result<Subscriber<()>> {
    debug!(pattern = %pattern, "declaring graph liveliness subscriber");
    let sub = session
        .liveliness()
        .declare_subscriber(pattern)
        .history(true)
        .callback({
            let graph_store = graph_store.clone();
            move |sample| {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                let sample_kind = sample.kind();
                debug!(
                    liveliness_key = %key_expr.0,
                    kind = ?sample_kind,
                    "received graph liveliness token"
                );

                match sample_kind {
                    SampleKind::Put => match parse_liveliness(&key_expr) {
                        Ok(entity) => {
                            graph_store.insert(key_expr, entity);
                        }
                        Err(error) => {
                            warn!(
                                liveliness_key = %key_expr.0,
                                error = ?error,
                                "failed to parse liveliness key; ignoring remote entity"
                            );
                        }
                    },
                    SampleKind::Delete => {
                        graph_store.remove(&key_expr);
                    }
                }
            }
        })
        .await
        .map_err(|source| crate::Error::zenoh("declare graph liveliness subscriber", source))?;

    Ok(sub)
}
