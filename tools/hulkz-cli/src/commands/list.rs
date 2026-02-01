//! List command implementations.

use hulkz::{PublisherInfo, Session};
use serde::Serialize;
use std::fmt;

use crate::output::OutputFormat;

/// Lists all nodes in the namespace.
pub async fn nodes(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Give time for discovery to settle
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let nodes = session.list_nodes().await?;
    format.print_list("NODES", namespace, &nodes);

    Ok(())
}

/// Lists all publishers in the namespace, optionally filtered by node.
pub async fn publishers(
    namespace: &str,
    node_filter: Option<&str>,
    format: OutputFormat,
) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Give time for discovery to settle
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let publishers = session.list_publishers().await?;

    // Filter by node if specified
    let filtered: Vec<_> = if let Some(node) = node_filter {
        publishers.into_iter().filter(|p| p.node == node).collect()
    } else {
        publishers
    };

    // Convert to display format
    let display_items: Vec<PublisherDisplay> =
        filtered.iter().map(PublisherDisplay::from).collect();

    format.print_list("PUBLISHERS", namespace, &display_items);

    Ok(())
}

/// Lists all sessions in the namespace.
pub async fn sessions(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Give time for discovery to settle
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let sessions = session.list_sessions().await?;
    format.print_list("SESSIONS", namespace, &sessions);

    Ok(())
}

/// Display wrapper for PublisherInfo with nice formatting.
#[derive(Serialize)]
struct PublisherDisplay {
    node: String,
    scope: String,
    path: String,
}

impl From<&PublisherInfo> for PublisherDisplay {
    fn from(info: &PublisherInfo) -> Self {
        Self {
            node: info.node.clone(),
            scope: format!("{}", info.scope),
            path: info.path.clone(),
        }
    }
}

impl fmt::Display for PublisherDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<16} {:<8} {}", self.node, self.scope, self.path)
    }
}
