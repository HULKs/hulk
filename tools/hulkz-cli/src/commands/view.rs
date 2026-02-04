//! View command - subscribe to the view plane and print messages.

use clap::Args;
use hulkz::Session;
use serde_json::Value;

use crate::output::OutputFormat;

/// Arguments for the view command.
#[derive(Args)]
pub struct ViewArgs {
    /// Topic to subscribe to (e.g., "camera/front", "/fleet_status", "~/debug")
    pub topic: String,

    /// Exit after receiving N messages
    #[arg(long)]
    pub count: Option<usize>,
}

/// Runs the view command.
pub async fn run(namespace: &str, args: ViewArgs, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;
    let node = session.create_node("hulkz-cli").build().await?;

    let topic: &str = &args.topic;

    // Subscribe to view plane (JSON) for CLI introspection
    let mut subscriber = node.subscribe::<Value>(topic).view().build().await?;

    if matches!(format, OutputFormat::Human) {
        println!(
            "Subscribing to: {} (namespace: {}, plane: view)",
            args.topic, namespace
        );
        println!("(Press Ctrl+C to exit)");
        println!();
    }

    let mut received = 0usize;

    loop {
        let message = subscriber.recv_async().await?;
        format.print_event("message", &message.payload);

        received += 1;
        if let Some(count) = args.count {
            if received >= count {
                break;
            }
        }
    }

    Ok(())
}
