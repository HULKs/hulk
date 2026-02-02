//! Hulkz CLI - Command-line tool for interacting with the hulkz middleware.

use clap::{Parser, Subcommand};

mod commands;
mod output;

use commands::{graph, info, list, param, view, watch};
use output::OutputFormat;

/// Hulkz CLI - Introspection and debugging tool for hulkz middleware
#[derive(Parser)]
#[command(name = "hulkz")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Namespace to operate in
    #[arg(short, long, env = "HULKZ_NAMESPACE", default_value = "default")]
    namespace: String,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List resources (nodes, publishers, sessions)
    List {
        #[command(subcommand)]
        resource: ListResource,
    },
    /// Watch for resource changes
    Watch {
        #[command(subcommand)]
        resource: WatchResource,
    },
    /// Subscribe to a topic and print messages (view plane, JSON)
    View(view::ViewArgs),
    /// Get or set parameters
    Param {
        #[command(subcommand)]
        action: ParamAction,
    },
    /// Show information about a topic
    Info(info::InfoArgs),
    /// Show network topology overview
    Graph,
}

#[derive(Subcommand)]
enum ListResource {
    /// List all nodes in the namespace
    Nodes,
    /// List all publishers in the namespace
    Publishers {
        /// Filter by node name
        #[arg(long)]
        node: Option<String>,
    },
    /// List all sessions in the namespace
    Sessions,
}

#[derive(Subcommand)]
enum WatchResource {
    /// Watch for node join/leave events
    Nodes,
    /// Watch for publisher advertise/unadvertise events
    Publishers,
    /// Watch for session join/leave events
    Sessions,
}

#[derive(Subcommand)]
enum ParamAction {
    /// List all parameters
    List(param::ListArgs),
    /// Get a parameter value
    Get(param::GetArgs),
    /// Set a parameter value
    Set(param::SetArgs),
}

#[tokio::main]
async fn main() -> hulkz::Result<()> {
    let cli = Cli::parse();
    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    match cli.command {
        Commands::List { resource } => match resource {
            ListResource::Nodes => list::nodes(&cli.namespace, format).await?,
            ListResource::Publishers { node } => {
                list::publishers(&cli.namespace, node.as_deref(), format).await?
            }
            ListResource::Sessions => list::sessions(&cli.namespace, format).await?,
        },
        Commands::Watch { resource } => match resource {
            WatchResource::Nodes => watch::nodes(&cli.namespace, format).await?,
            WatchResource::Publishers => watch::publishers(&cli.namespace, format).await?,
            WatchResource::Sessions => watch::sessions(&cli.namespace, format).await?,
        },
        Commands::View(args) => view::run(&cli.namespace, args, format).await?,
        Commands::Param { action } => match action {
            ParamAction::List(args) => param::list(&cli.namespace, args, format).await?,
            ParamAction::Get(args) => param::get(&cli.namespace, args, format).await?,
            ParamAction::Set(args) => param::set(&cli.namespace, args, format).await?,
        },
        Commands::Info(args) => info::run(&cli.namespace, args, format).await?,
        Commands::Graph => graph::run(&cli.namespace, format).await?,
    }

    Ok(())
}
