use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::{Notify, watch};
use tracing::{debug, warn};
use zenoh::{Session, pubsub::Subscriber, sample::SampleKind};

use crate::{Result, entity::LivelinessKE};

use super::{GraphOptions, state::GraphData};
use ros_z_protocol::format::parse_liveliness;

#[derive(Debug)]
enum LivelinessEvent {
    Put(LivelinessKE, Box<crate::entity::Entity>),
    Delete(LivelinessKE),
}

#[derive(Debug, Default)]
struct InitialHydrationState {
    complete: bool,
    pending_events: Vec<LivelinessEvent>,
}

fn apply_liveliness_event(graph_data: &Arc<Mutex<GraphData>>, event: LivelinessEvent) -> bool {
    match event {
        LivelinessEvent::Put(key_expr, entity) => graph_data.lock().insert(key_expr, *entity),
        LivelinessEvent::Delete(key_expr) => graph_data.lock().remove(&key_expr),
    }
}

fn record_live_liveliness_event(
    hydration_state: &Arc<Mutex<InitialHydrationState>>,
    graph_data: &Arc<Mutex<GraphData>>,
    event: LivelinessEvent,
) -> bool {
    let mut hydration_state = hydration_state.lock();
    if !hydration_state.complete {
        hydration_state.pending_events.push(event);
        return false;
    }

    apply_liveliness_event(graph_data, event)
}

fn complete_initial_hydration(
    hydration_state: &Arc<Mutex<InitialHydrationState>>,
    graph_data: &Arc<Mutex<GraphData>>,
) -> bool {
    let mut hydration_state = hydration_state.lock();
    hydration_state.complete = true;

    let mut changed = false;
    for event in hydration_state.pending_events.drain(..) {
        changed |= apply_liveliness_event(graph_data, event);
    }
    changed
}

pub(super) async fn install_liveliness(
    session: &Session,
    pattern: &str,
    options: &GraphOptions,
    graph_data: Arc<Mutex<GraphData>>,
    change_notify: Arc<Notify>,
    change_revision: watch::Sender<u64>,
) -> Result<Subscriber<()>> {
    debug!(pattern = %pattern, "declaring graph liveliness subscriber");
    let initial_hydration = Arc::new(Mutex::new(InitialHydrationState::default()));
    let sub = session
        .liveliness()
        .declare_subscriber(pattern)
        .callback({
            let graph_data = graph_data.clone();
            let initial_hydration = initial_hydration.clone();
            let change_notify = change_notify.clone();
            let change_revision = change_revision.clone();
            move |sample| {
                let key_expr = LivelinessKE(sample.key_expr().to_owned());
                let sample_kind = sample.kind();
                debug!(
                    liveliness_key = %key_expr.0,
                    kind = ?sample_kind,
                    "received graph liveliness token"
                );

                let event = match sample_kind {
                    SampleKind::Put => match parse_liveliness(&key_expr) {
                        Ok(entity) => Some(LivelinessEvent::Put(key_expr, Box::new(entity))),
                        Err(error) => {
                            warn!(
                                liveliness_key = %key_expr.0,
                            error = ?error,
                                "failed to parse liveliness key; ignoring remote entity"
                            );
                            None
                        }
                    },
                    SampleKind::Delete => Some(LivelinessEvent::Delete(key_expr)),
                };

                let changed = event.is_some_and(|event| {
                    record_live_liveliness_event(&initial_hydration, &graph_data, event)
                });

                if changed {
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
    if complete_initial_hydration(&initial_hydration, &graph_data) {
        change_revision.send_modify(|revision| *revision = revision.saturating_add(1));
        change_notify.notify_waiters();
    }

    Ok(sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, NodeEntity};

    fn node_entity(id: usize, name: &str) -> Entity {
        Entity::Node(NodeEntity::new(
            zenoh::session::ZenohId::default(),
            id,
            name.to_string(),
            String::new(),
        ))
    }

    fn key_for(entity: &Entity) -> LivelinessKE {
        entity
            .liveliness_key_expr()
            .expect("test entity should format as liveliness key")
    }

    #[test]
    fn queued_live_delete_overrides_initial_query_insert() {
        let graph_data = Arc::new(Mutex::new(GraphData::new()));
        let hydration_state = Arc::new(Mutex::new(InitialHydrationState::default()));
        let entity = node_entity(1, "queued_delete");
        let key_expr = key_for(&entity);

        assert!(!record_live_liveliness_event(
            &hydration_state,
            &graph_data,
            LivelinessEvent::Delete(key_expr.clone())
        ));

        graph_data.lock().insert(key_expr, entity);
        assert_eq!(graph_data.lock().entities().count(), 1);

        assert!(complete_initial_hydration(&hydration_state, &graph_data));
        assert_eq!(graph_data.lock().entities().count(), 0);
    }
}
