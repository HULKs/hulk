//! View command - subscribe to the view plane and print messages.

use clap::Args;
use color_eyre::{eyre::bail, Result};
use hulkz::{Session, TopicExpression};
use serde_json::Value;

/// Arguments for the view command.
#[derive(Args)]
pub struct ViewArgs {
    /// Topic to subscribe to (e.g., "camera/front", "/fleet_status", "~/debug")
    pub topic: String,

    /// Node name (required for private topics: "~/...")
    #[arg(long)]
    pub node: Option<String>,

    /// Exit after receiving N messages
    #[arg(long)]
    pub count: Option<usize>,
}

/// Runs the view command.
pub async fn run(namespace: &str, args: ViewArgs) -> Result<()> {
    let session = Session::create(namespace).await?;
    let node = session.create_node("hulkz-cli").build().await?;

    let topic_expression = TopicExpression::parse(args.topic.as_str())?;
    if topic_expression.as_str().starts_with("~/") && args.node.is_none() {
        bail!("Private topic '{}' requires --node argument", args.topic);
    }
    topic_expression.resolve(namespace, args.node.as_deref())?;

    // Subscribe to view plane (JSON) for CLI introspection
    let mut builder = node.subscribe::<Value>(topic_expression).view();
    if let Some(node_name) = args.node.as_deref() {
        builder = builder.on_node(node_name);
    }
    let mut subscriber = builder.build().await?;

    println!(
        "Subscribing to: {} (namespace: {}, plane: view)",
        args.topic, namespace
    );
    println!("(Press Ctrl+C to exit)");
    println!();

    let mut received = 0usize;

    loop {
        let message = subscriber.recv_async().await?;
        println!("{}", serde_json::to_string_pretty(&message)?);

        received += 1;
        if let Some(count) = args.count {
            if received >= count {
                break;
            }
        }
    }

    Ok(())
}
