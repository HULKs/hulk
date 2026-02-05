//! Parameter command - get, set, and list parameter values.

use clap::Args;
use color_eyre::{eyre::Context, Result};
use hulkz::Session;

/// Arguments for param get command.
#[derive(Args)]
pub struct GetArgs {
    /// Parameter path (e.g., "max_speed", "/fleet_id", "~/debug_level")
    pub path: String,

    /// Node name (required for private parameters)
    #[arg(long)]
    pub node: Option<String>,
}

/// Arguments for param set command.
#[derive(Args)]
pub struct SetArgs {
    /// Parameter path (e.g., "max_speed", "/fleet_id", "~/debug_level")
    pub path: String,

    /// JSON value to set (use quotes for strings, e.g., '"hello"')
    #[arg(allow_hyphen_values = true)]
    pub value: String,

    /// Node name (required for private parameters)
    #[arg(long)]
    pub node: Option<String>,
}

/// Arguments for param list command.
#[derive(Args)]
pub struct ListArgs {
    /// Filter by node name (only shows private parameters for this node)
    #[arg(long)]
    pub node: Option<String>,
}

/// Lists all parameters.
pub async fn list(namespace: &str, args: ListArgs) -> Result<()> {
    let session = Session::create(namespace).await?;
    let mut parameters = session.graph().parameters().list().await?;

    // Filter by node if specified
    if let Some(node_filter) = &args.node {
        parameters.retain(|p| p.node == *node_filter);
    }

    // Sort by scope (global, local, private) then by path
    parameters.sort_by(|a, b| {
        let scope_ord = |s: &hulkz::Scope| match s {
            hulkz::Scope::Global => 0,
            hulkz::Scope::Local => 1,
            hulkz::Scope::Private => 2,
        };
        scope_ord(&a.scope)
            .cmp(&scope_ord(&b.scope))
            .then_with(|| a.path.cmp(&b.path))
    });

    if parameters.is_empty() {
        println!("No parameters found");
    } else {
        println!("Parameters in namespace '{}':", namespace);
        println!();
        for param in &parameters {
            println!("  {} (node: {})", param.display_path(), param.node);
        }
        println!();
        println!("Total: {} parameter(s)", parameters.len());
    }

    Ok(())
}

/// Gets a parameter value.
pub async fn get(namespace: &str, args: GetArgs) -> Result<()> {
    let session = Session::create(namespace)
        .await
        .wrap_err("failed to create session")?;

    let mut builder = session.parameter(args.path.as_str());
    if let Some(node) = &args.node {
        builder = builder.on_node(node);
    }

    let mut replies = builder
        .get::<serde_json::Value>()
        .await
        .wrap_err("failed to query parameter")?;

    while let Some(reply) = replies.recv_async().await {
        let value = reply.wrap_err("failed to receive parameter value")?;
        println!(
            "{}: {}",
            args.path,
            serde_json::to_string_pretty(&value).wrap_err("failed to serialize parameter value")?
        );
    }

    Ok(())
}

/// Sets a parameter value.
pub async fn set(namespace: &str, args: SetArgs) -> Result<()> {
    let session = Session::create(namespace)
        .await
        .wrap_err("failed to create session")?;

    let value: serde_json::Value =
        serde_json::from_str(&args.value).wrap_err("failed to parse value as JSON")?;

    let mut builder = session.parameter(args.path.as_str());
    if let Some(node) = &args.node {
        builder = builder.on_node(node);
    }

    let mut replies = builder.set(&value).await?;
    while let Some(reply) = replies.recv_async().await {
        reply.wrap_err("failed to set parameter value")?;
        println!("Set {} = {}", args.path, args.value);
    }

    Ok(())
}
