//! Info command - show information about a topic.

use clap::Args;
use hulkz::{ScopedPath, Session};
use serde::Serialize;

use crate::output::OutputFormat;

/// Arguments for the info command.
#[derive(Args)]
pub struct InfoArgs {
    /// Topic to get info about (e.g., "camera/front", "/fleet_status")
    pub topic: String,
}

#[derive(Serialize)]
struct TopicInfo {
    topic: String,
    scope: String,
    path: String,
    publishers: Vec<PublisherMatch>,
}

#[derive(Serialize)]
struct PublisherMatch {
    node: String,
}

/// Runs the info command.
pub async fn run(namespace: &str, args: InfoArgs, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Parse the topic
    let scoped_path = ScopedPath::parse(&args.topic);

    // Give time for discovery
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Find publishers for this topic
    let all_publishers = session.list_publishers().await?;
    let matching_publishers: Vec<_> = all_publishers
        .iter()
        .filter(|p| p.path == scoped_path.path() && p.scope == scoped_path.scope())
        .collect();

    let info = TopicInfo {
        topic: args.topic.clone(),
        scope: format!("{}", scoped_path.scope()),
        path: scoped_path.path().to_string(),
        publishers: matching_publishers
            .iter()
            .map(|p| PublisherMatch {
                node: p.node.clone(),
            })
            .collect(),
    };

    if matches!(format, OutputFormat::Human) {
        println!("TOPIC INFO");
        println!("  Topic:     {}", info.topic);
        println!("  Scope:     {}", info.scope);
        println!("  Path:      {}", info.path);
        println!();
        println!("PUBLISHERS ({})", info.publishers.len());
        if info.publishers.is_empty() {
            println!("  (none)");
        } else {
            for p in &info.publishers {
                println!("  {}", p.node);
            }
        }
    } else {
        println!("{}", serde_json::to_string(&info).unwrap_or_default());
    }

    Ok(())
}
