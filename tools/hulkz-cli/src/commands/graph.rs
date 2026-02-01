//! Graph command - show network topology overview.

use std::collections::HashMap;

use hulkz::Session;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Serialize)]
struct NetworkGraph {
    namespace: String,
    sessions: usize,
    nodes: Vec<NodeSummary>,
}

#[derive(Serialize)]
struct NodeSummary {
    name: String,
    publishers: Vec<String>,
}

/// Runs the graph command.
pub async fn run(namespace: &str, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Give time for discovery
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Gather data
    let sessions = session.list_sessions().await?;
    let nodes = session.list_nodes().await?;
    let publishers = session.list_publishers().await?;

    // Group publishers by node
    let mut node_publishers: HashMap<String, Vec<String>> = HashMap::new();
    for node in &nodes {
        node_publishers.insert(node.clone(), Vec::new());
    }
    for pub_info in &publishers {
        node_publishers
            .entry(pub_info.node.clone())
            .or_default()
            .push(format!("{}/{}", pub_info.scope, pub_info.path));
    }

    let graph = NetworkGraph {
        namespace: namespace.to_string(),
        sessions: sessions.len(),
        nodes: nodes
            .iter()
            .map(|name| NodeSummary {
                name: name.clone(),
                publishers: node_publishers.get(name).cloned().unwrap_or_default(),
            })
            .collect(),
    };

    if matches!(format, OutputFormat::Human) {
        println!("NETWORK GRAPH");
        println!("  Namespace: {}", graph.namespace);
        println!("  Sessions:  {}", graph.sessions);
        println!();
        println!("NODES ({})", graph.nodes.len());
        if graph.nodes.is_empty() {
            println!("  (none)");
        } else {
            for node in &graph.nodes {
                println!(
                    "  {} ({} publishers)",
                    node.name,
                    node.publishers.len()
                );
                for pub_topic in &node.publishers {
                    println!("    - {}", pub_topic);
                }
            }
        }
    } else {
        println!("{}", serde_json::to_string(&graph).unwrap_or_default());
    }

    Ok(())
}
