//! Info command - show information about a topic.

use clap::Args;
use color_eyre::Result;
use hulkz::{ScopedPath, Session};
use serde::Serialize;

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
pub async fn run(namespace: &str, args: InfoArgs) -> Result<()> {
    let session = Session::create(namespace).await?;

    // Parse the topic
    let scoped_path: ScopedPath = args.topic.as_str().into();

    // Find publishers for this topic using new Graph API
    let all_publishers = session.graph().publishers().list().await?;
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

    Ok(())
}
