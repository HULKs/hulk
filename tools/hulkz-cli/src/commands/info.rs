//! Info command - show information about a topic.

use clap::Args;
use color_eyre::Result;
use hulkz::{Session, TopicExpression};
use serde::Serialize;

/// Arguments for the info command.
#[derive(Args)]
pub struct InfoArgs {
    /// Topic to get info about (e.g., "camera/front", "/fleet_status")
    pub topic: String,

    /// Node name for private expressions ("~/...")
    #[arg(long)]
    pub node: Option<String>,
}

#[derive(Serialize)]
struct TopicInfo {
    topic_expression: String,
    resolved_topic: String,
    publishers: Vec<PublisherMatch>,
}

#[derive(Serialize)]
struct PublisherMatch {
    node: String,
}

/// Runs the info command.
pub async fn run(namespace: &str, args: InfoArgs) -> Result<()> {
    let session = Session::create(namespace).await?;

    let topic_expression = TopicExpression::parse(args.topic.as_str())?;
    let resolved_topic = topic_expression.resolve(namespace, args.node.as_deref())?;

    // Find publishers for this topic using new Graph API
    let all_publishers = session.graph().publishers().list().await?;
    let matching_publishers: Vec<_> = all_publishers
        .iter()
        .filter(|p| p.topic == resolved_topic)
        .collect();

    let info = TopicInfo {
        topic_expression: args.topic.clone(),
        resolved_topic,
        publishers: matching_publishers
            .iter()
            .map(|p| PublisherMatch {
                node: p.node.clone(),
            })
            .collect(),
    };

    println!("TOPIC INFO");
    println!("  Expression: {}", info.topic_expression);
    println!("  Resolved:   {}", info.resolved_topic);
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
