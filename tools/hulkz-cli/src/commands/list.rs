//! List command implementations.

use hulkz::{PublisherInfo, Session};
use serde::Serialize;
use std::fmt;

/// Lists all nodes in the namespace.
pub async fn nodes(namespace: &str) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Get nodes as NodeInfo, extract just names for display
    let nodes = session.graph().nodes().list().await?;
    let node_names: Vec<String> = nodes.into_iter().map(|n| n.name).collect();
    println!("NODES in namespace '{}':", namespace);
    if node_names.is_empty() {
        println!("  (none)");
    } else {
        for node in node_names {
            println!("  {}", node);
        }
    }

    Ok(())
}

/// Lists all publishers in the namespace, optionally filtered by node.
pub async fn publishers(namespace: &str, node_filter: Option<&str>) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    let publishers = session.graph().publishers().list().await?;

    // Filter by node if specified
    let filtered: Vec<_> = if let Some(node) = node_filter {
        publishers.into_iter().filter(|p| p.node == node).collect()
    } else {
        publishers
    };

    // Convert to display format
    let display_items: Vec<PublisherDisplay> =
        filtered.iter().map(PublisherDisplay::from).collect();

    println!("PUBLISHERS in namespace '{}':", namespace);
    if display_items.is_empty() {
        println!("  (none)");
    } else {
        println!("{:<16} {:<8} PATH", "Node", "Scope");
        println!("{}", "-".repeat(40));
        for item in display_items {
            println!("{}", item);
        }
    }

    Ok(())
}

/// Lists all sessions in the namespace.
pub async fn sessions(namespace: &str) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Get sessions as SessionInfo, extract just IDs for display
    let sessions = session.graph().sessions().list().await?;
    let session_ids: Vec<String> = sessions.into_iter().map(|s| s.id).collect();

    println!("SESSIONS in namespace '{}':", namespace);
    if session_ids.is_empty() {
        println!("  (none)");
    } else {
        for session in session_ids {
            println!("  {}", session);
        }
    }

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
