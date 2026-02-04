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

    // Gather data using new fluent Graph API
    let sessions = session.graph().sessions().list().await?;
    let nodes = session.graph().nodes().list().await?;
    let publishers = session.graph().publishers().list().await?;

    // Group publishers by node (keyed by node name)
    let mut node_publishers: HashMap<String, Vec<String>> = HashMap::new();
    for node_info in &nodes {
        node_publishers.insert(node_info.name.clone(), Vec::new());
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
            .map(|node_info| NodeSummary {
                name: node_info.name.clone(),
                publishers: node_publishers
                    .get(&node_info.name)
                    .cloned()
                    .unwrap_or_default(),
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
                println!("  {} ({} publishers)", node.name, node.publishers.len());
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
