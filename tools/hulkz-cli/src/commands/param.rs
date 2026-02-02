//! Parameter command - get, set, and list parameter values.

use clap::Args;
use hulkz::Session;
use serde::Serialize;

use crate::output::OutputFormat;

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

/// JSON output for parameter list.
#[derive(Serialize)]
struct ParameterListItem {
    path: String,
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<String>,
}

/// Lists all parameters.
pub async fn list(namespace: &str, args: ListArgs, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;
    let mut parameters = session.list_parameters().await?;

    // Filter by node if specified
    if let Some(ref node_filter) = args.node {
        parameters.retain(|p| p.node.as_deref() == Some(node_filter.as_str()));
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

    if matches!(format, OutputFormat::Human) {
        if parameters.is_empty() {
            println!("No parameters found");
        } else {
            println!("Parameters in namespace '{}':", namespace);
            println!();
            for param in &parameters {
                let node_info = param
                    .node
                    .as_ref()
                    .map(|n| format!(" (node: {})", n))
                    .unwrap_or_default();
                println!("  {}{}", param.display_path(), node_info);
            }
            println!();
            println!("Total: {} parameter(s)", parameters.len());
        }
    } else {
        let items: Vec<ParameterListItem> = parameters
            .iter()
            .map(|p| ParameterListItem {
                path: p.display_path(),
                scope: p.scope.as_str().to_string(),
                node: p.node.clone(),
            })
            .collect();
        println!("{}", serde_json::to_string(&items).unwrap_or_default());
    }

    Ok(())
}

/// Gets a parameter value.
pub async fn get(namespace: &str, args: GetArgs, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Build the parameter query
    let mut builder = session.parameter(&args.path);
    if let Some(ref node) = args.node {
        builder = builder.on_node(node);
    }

    // Query the parameter
    match builder.get().await {
        Ok(Some(value)) => {
            if matches!(format, OutputFormat::Human) {
                println!(
                    "{}: {}",
                    args.path,
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            } else {
                println!("{}", serde_json::to_string(&value).unwrap_or_default());
            }
        }
        Ok(None) => {
            if matches!(format, OutputFormat::Human) {
                println!("Parameter '{}' not found", args.path);
            } else {
                println!("null");
            }
        }
        Err(hulkz::Error::NodeRequiredForPrivate) => {
            if matches!(format, OutputFormat::Human) {
                eprintln!(
                    "Private parameter '{}' requires --node argument",
                    args.path
                );
            } else {
                println!(
                    r#"{{"error":"node required for private parameter","path":"{}"}}"#,
                    args.path
                );
            }
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

/// Sets a parameter value.
pub async fn set(namespace: &str, args: SetArgs, format: OutputFormat) -> hulkz::Result<()> {
    let session = Session::create(namespace).await?;

    // Parse the value as JSON
    let value: serde_json::Value =
        serde_json::from_str(&args.value).map_err(hulkz::Error::JsonDeserialize)?;

    // Build the parameter query
    let mut builder = session.parameter(&args.path);
    if let Some(ref node) = args.node {
        builder = builder.on_node(node);
    }

    // Set the parameter
    match builder.set(&value).await {
        Ok(()) => {
            // Success
            if matches!(format, OutputFormat::Human) {
                println!("Set {} = {}", args.path, args.value);
            } else {
                println!(
                    r#"{{"success":true,"path":"{}","value":{}}}"#,
                    args.path, args.value
                );
            }
        }
        Err(hulkz::Error::ParameterRejected(reasons)) => {
            // Parameter(s) rejected the value (e.g., validation failed)
            let error_msg = reasons.join("; ");
            if matches!(format, OutputFormat::Human) {
                eprintln!("Failed to set '{}': {}", args.path, error_msg);
            } else {
                println!(
                    r#"{{"success":false,"path":"{}","error":"{}"}}"#,
                    args.path,
                    error_msg.replace('"', "\\\"")
                );
            }
        }
        Err(hulkz::Error::ParameterNotFound(_)) => {
            if matches!(format, OutputFormat::Human) {
                eprintln!(
                    "Parameter '{}' not found (no node is serving this parameter)",
                    args.path
                );
            } else {
                println!(
                    r#"{{"success":false,"path":"{}","error":"not found"}}"#,
                    args.path
                );
            }
        }
        Err(hulkz::Error::NodeRequiredForPrivate) => {
            if matches!(format, OutputFormat::Human) {
                eprintln!(
                    "Private parameter '{}' requires --node argument",
                    args.path
                );
            } else {
                println!(
                    r#"{{"success":false,"path":"{}","error":"node required for private parameter"}}"#,
                    args.path
                );
            }
        }
        Err(e) => return Err(e),
    }

    Ok(())
}
