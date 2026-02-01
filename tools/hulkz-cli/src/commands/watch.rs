//! Watch command implementations.

use hulkz::{NodeEvent, PublisherEvent, PublisherInfo, Session, SessionEvent};
use serde::Serialize;

use crate::output::OutputFormat;

/// Watches for node join/leave events.
pub async fn nodes(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.watch_nodes().await?;
    tokio::spawn(driver);

    if matches!(format, OutputFormat::Human) {
        println!("Watching for node events in namespace: {}", namespace);
        println!("(Press Ctrl+C to exit)");
        println!();
    }

    while let Some(event) = watcher.recv().await {
        match &event {
            NodeEvent::Joined(name) => {
                format.print_event("node.joined", &NodeEventData { node: name.clone() });
            }
            NodeEvent::Left(name) => {
                format.print_event("node.left", &NodeEventData { node: name.clone() });
            }
        }
    }

    Ok(())
}

/// Watches for publisher advertise/unadvertise events.
pub async fn publishers(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.watch_publishers().await?;
    tokio::spawn(driver);

    if matches!(format, OutputFormat::Human) {
        println!(
            "Watching for publisher events in namespace: {}",
            namespace
        );
        println!("(Press Ctrl+C to exit)");
        println!();
    }

    while let Some(event) = watcher.recv().await {
        match &event {
            PublisherEvent::Advertised(info) => {
                format.print_event("publisher.advertised", &PublisherEventData::from(info));
            }
            PublisherEvent::Unadvertised(info) => {
                format.print_event("publisher.unadvertised", &PublisherEventData::from(info));
            }
        }
    }

    Ok(())
}

/// Watches for session join/leave events.
pub async fn sessions(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.watch_sessions().await?;
    tokio::spawn(driver);

    if matches!(format, OutputFormat::Human) {
        println!("Watching for session events in namespace: {}", namespace);
        println!("(Press Ctrl+C to exit)");
        println!();
    }

    while let Some(event) = watcher.recv().await {
        match &event {
            SessionEvent::Joined(id) => {
                format.print_event(
                    "session.joined",
                    &SessionEventData {
                        session_id: id.clone(),
                    },
                );
            }
            SessionEvent::Left(id) => {
                format.print_event(
                    "session.left",
                    &SessionEventData {
                        session_id: id.clone(),
                    },
                );
            }
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct NodeEventData {
    node: String,
}

#[derive(Serialize)]
struct SessionEventData {
    session_id: String,
}

#[derive(Serialize)]
struct PublisherEventData {
    node: String,
    scope: String,
    path: String,
}

impl From<&PublisherInfo> for PublisherEventData {
    fn from(info: &PublisherInfo) -> Self {
        Self {
            node: info.node.clone(),
            scope: format!("{}", info.scope),
            path: info.path.clone(),
        }
    }
}
