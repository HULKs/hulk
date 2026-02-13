use std::future::Future;

use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz::{GraphEvent, ParameterInfo, PublisherInfo, Session, SessionInfo, Watcher};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tracing::trace;

use crate::model::{
    DiscoveredParameter, DiscoveredPublisher, DiscoveredSession, WorkerEvent, WorkerEventEnvelope,
};

use super::{
    commands::send_event,
    streams::{to_discovered_parameter, to_discovered_publisher, to_discovered_session},
};

pub(super) enum DiscoveryEvent {
    PublisherJoined(DiscoveredPublisher),
    PublisherLeft(DiscoveredPublisher),
    ParameterJoined(DiscoveredParameter),
    ParameterLeft(DiscoveredParameter),
    SessionJoined(DiscoveredSession),
    SessionLeft(DiscoveredSession),
    WatchFault(String),
}

pub(super) struct DiscoveryState {
    pub(super) cancel: CancellationToken,
    pub(super) tasks: Vec<tokio::task::JoinHandle<()>>,
    pub(super) publishers: Vec<DiscoveredPublisher>,
    pub(super) parameters: Vec<DiscoveredParameter>,
    pub(super) sessions: Vec<DiscoveredSession>,
}

impl DiscoveryState {
    pub(super) fn new() -> Self {
        Self {
            cancel: CancellationToken::new(),
            tasks: Vec::new(),
            publishers: Vec::new(),
            parameters: Vec::new(),
            sessions: Vec::new(),
        }
    }
}

pub(super) async fn restart_discovery_watchers(
    session: &Session,
    namespace: Option<&str>,
    discovery_event_tx: &Sender<DiscoveryEvent>,
    worker_event_tx: &Sender<WorkerEventEnvelope>,
    discovery: &mut DiscoveryState,
) -> Result<()> {
    stop_discovery_watchers(&mut discovery.cancel, &mut discovery.tasks);
    discovery.cancel = CancellationToken::new();
    reconcile_discovery_snapshot(
        session,
        namespace,
        worker_event_tx,
        &mut discovery.publishers,
        &mut discovery.parameters,
        &mut discovery.sessions,
    )
    .await?;
    start_discovery_watchers(
        session,
        namespace,
        discovery_event_tx,
        &discovery.cancel,
        &mut discovery.tasks,
    )
    .await
}

pub(super) fn stop_discovery_watchers(
    discovery_cancel: &mut CancellationToken,
    discovery_tasks: &mut Vec<tokio::task::JoinHandle<()>>,
) {
    discovery_cancel.cancel();
    for task in discovery_tasks.drain(..) {
        task.abort();
    }
}

pub(super) async fn start_discovery_watchers(
    session: &Session,
    namespace: Option<&str>,
    discovery_event_tx: &Sender<DiscoveryEvent>,
    discovery_cancel: &CancellationToken,
    discovery_tasks: &mut Vec<tokio::task::JoinHandle<()>>,
) -> Result<()> {
    if let Some(namespace) = namespace {
        let (publisher_watcher, publisher_driver) = session
            .graph()
            .in_namespace(namespace)
            .publishers()
            .watch()
            .await
            .wrap_err_with(|| {
                format!("failed to start publisher discovery watch in namespace {namespace}")
            })?;
        discovery_tasks.push(spawn_publisher_watcher_task(
            publisher_watcher,
            discovery_event_tx.clone(),
            discovery_cancel.clone(),
        ));
        discovery_tasks.push(spawn_watch_driver_task(
            publisher_driver,
            "publisher",
            discovery_event_tx.clone(),
            discovery_cancel.clone(),
        ));

        let (parameter_watcher, parameter_driver) = session
            .graph()
            .in_namespace(namespace)
            .parameters()
            .watch()
            .await
            .wrap_err_with(|| {
                format!("failed to start parameter discovery watch in namespace {namespace}")
            })?;
        discovery_tasks.push(spawn_parameter_watcher_task(
            parameter_watcher,
            discovery_event_tx.clone(),
            discovery_cancel.clone(),
        ));
        discovery_tasks.push(spawn_watch_driver_task(
            parameter_driver,
            "parameter",
            discovery_event_tx.clone(),
            discovery_cancel.clone(),
        ));
    }

    let (session_watcher, session_driver) = if let Some(namespace) = namespace {
        session
            .graph()
            .in_namespace(namespace)
            .sessions()
            .watch()
            .await
            .wrap_err_with(|| {
                format!("failed to start session discovery watch in namespace {namespace}")
            })?
    } else {
        session
            .graph()
            .sessions()
            .watch()
            .await
            .wrap_err("failed to start global session discovery watch")?
    };
    discovery_tasks.push(spawn_session_watcher_task(
        session_watcher,
        discovery_event_tx.clone(),
        discovery_cancel.clone(),
    ));
    discovery_tasks.push(spawn_watch_driver_task(
        session_driver,
        "session",
        discovery_event_tx.clone(),
        discovery_cancel.clone(),
    ));

    Ok(())
}

fn spawn_publisher_watcher_task(
    mut watcher: Watcher<GraphEvent<PublisherInfo>>,
    discovery_event_tx: Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                event = watcher.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    let mapped = match event {
                        GraphEvent::Joined(info) => {
                            DiscoveryEvent::PublisherJoined(to_discovered_publisher(info))
                        }
                        GraphEvent::Left(info) => {
                            DiscoveryEvent::PublisherLeft(to_discovered_publisher(info))
                        }
                    };
                    if discovery_event_tx.send(mapped).await.is_err() {
                        break;
                    }
                }
            }
        }
    })
}

fn spawn_parameter_watcher_task(
    mut watcher: Watcher<GraphEvent<ParameterInfo>>,
    discovery_event_tx: Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                event = watcher.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    let mapped = match event {
                        GraphEvent::Joined(info) => {
                            DiscoveryEvent::ParameterJoined(to_discovered_parameter(info))
                        }
                        GraphEvent::Left(info) => {
                            DiscoveryEvent::ParameterLeft(to_discovered_parameter(info))
                        }
                    };
                    if discovery_event_tx.send(mapped).await.is_err() {
                        break;
                    }
                }
            }
        }
    })
}

fn spawn_session_watcher_task(
    mut watcher: Watcher<GraphEvent<SessionInfo>>,
    discovery_event_tx: Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                event = watcher.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    let mapped = match event {
                        GraphEvent::Joined(info) => {
                            DiscoveryEvent::SessionJoined(to_discovered_session(info))
                        }
                        GraphEvent::Left(info) => {
                            DiscoveryEvent::SessionLeft(to_discovered_session(info))
                        }
                    };
                    if discovery_event_tx.send(mapped).await.is_err() {
                        break;
                    }
                }
            }
        }
    })
}

fn spawn_watch_driver_task<F>(
    driver: F,
    kind: &'static str,
    discovery_event_tx: Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = hulkz::Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel.cancelled() => {}
            result = driver => {
                if let Err(error) = result {
                    let _ = discovery_event_tx
                        .send(DiscoveryEvent::WatchFault(format!(
                            "{kind} discovery watch failed: {error}"
                        )))
                        .await;
                }
            }
        }
    })
}

pub(super) async fn reconcile_discovery_snapshot(
    session: &Session,
    namespace: Option<&str>,
    event_tx: &Sender<WorkerEventEnvelope>,
    publishers: &mut Vec<DiscoveredPublisher>,
    parameters: &mut Vec<DiscoveredParameter>,
    sessions: &mut Vec<DiscoveredSession>,
) -> Result<()> {
    let (listed_publishers, listed_parameters, listed_sessions) =
        list_discovery_snapshot(session, namespace).await?;
    if *publishers == listed_publishers
        && *parameters == listed_parameters
        && *sessions == listed_sessions
    {
        return Ok(());
    }
    *publishers = listed_publishers;
    *parameters = listed_parameters;
    *sessions = listed_sessions;
    emit_discovery_snapshot(event_tx, publishers, parameters, sessions).await?;
    Ok(())
}

async fn list_discovery_snapshot(
    session: &Session,
    namespace: Option<&str>,
) -> Result<(
    Vec<DiscoveredPublisher>,
    Vec<DiscoveredParameter>,
    Vec<DiscoveredSession>,
)> {
    let publishers = if let Some(namespace) = namespace {
        let mut publishers = session
            .graph()
            .in_namespace(namespace)
            .publishers()
            .list()
            .await
            .wrap_err_with(|| {
                format!("failed to list discovered publishers in namespace {namespace}")
            })?
            .into_iter()
            .map(to_discovered_publisher)
            .collect::<Vec<_>>();
        publishers.sort();
        publishers.dedup();
        publishers
    } else {
        Vec::new()
    };

    let parameters = if let Some(namespace) = namespace {
        let mut parameters = session
            .graph()
            .in_namespace(namespace)
            .parameters()
            .list()
            .await
            .wrap_err_with(|| {
                format!("failed to list discovered parameters in namespace {namespace}")
            })?
            .into_iter()
            .map(to_discovered_parameter)
            .collect::<Vec<_>>();
        parameters.sort();
        parameters.dedup();
        parameters
    } else {
        Vec::new()
    };

    let mut sessions = if let Some(namespace) = namespace {
        session
            .graph()
            .in_namespace(namespace)
            .sessions()
            .list()
            .await
            .wrap_err_with(|| {
                format!("failed to list discovered sessions in namespace {namespace}")
            })?
            .into_iter()
            .map(to_discovered_session)
            .collect::<Vec<_>>()
    } else {
        session
            .graph()
            .sessions()
            .list()
            .await
            .wrap_err("failed to list discovered sessions globally")?
            .into_iter()
            .map(to_discovered_session)
            .collect::<Vec<_>>()
    };
    sessions.sort();
    sessions.dedup();

    Ok((publishers, parameters, sessions))
}

pub(super) async fn emit_discovery_snapshot(
    event_tx: &Sender<WorkerEventEnvelope>,
    publishers: &[DiscoveredPublisher],
    parameters: &[DiscoveredParameter],
    sessions: &[DiscoveredSession],
) -> Result<()> {
    trace!(
        publishers = publishers.len(),
        parameters = parameters.len(),
        sessions = sessions.len(),
        "publishing discovery snapshot"
    );
    send_event(
        event_tx,
        WorkerEvent::DiscoverySnapshot {
            publishers: publishers.to_vec(),
            parameters: parameters.to_vec(),
            sessions: sessions.to_vec(),
        },
    )
    .await
    .map_err(|_| eyre!("failed to send discovery snapshot: worker event channel closed"))?;
    Ok(())
}

pub(super) fn insert_discovered_entity<T: Ord>(entities: &mut Vec<T>, entity: T) -> bool {
    match entities.binary_search(&entity) {
        Ok(_) => false,
        Err(index) => {
            entities.insert(index, entity);
            true
        }
    }
}

pub(super) fn remove_discovered_entity<T: Ord>(entities: &mut Vec<T>, entity: &T) -> bool {
    match entities.binary_search(entity) {
        Ok(index) => {
            entities.remove(index);
            true
        }
        Err(_) => false,
    }
}
