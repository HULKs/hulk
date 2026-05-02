use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ListTarget {
    Topics,
    Nodes,
    Services,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum InfoTarget {
    Topic,
    Service,
    Node,
}

#[derive(Debug, Parser)]
#[command(name = "rosz")]
#[command(about = "Scriptable command-line companion to ros-z")]
pub struct Cli {
    /// Zenoh router address
    #[arg(long, default_value = "tcp/127.0.0.1:7447", global = true)]
    pub router: String,

    /// Emit JSON output when supported
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List graph entities
    List {
        #[arg(value_enum)]
        target: ListTarget,
    },
    /// Watch graph changes continuously
    Watch,
    /// Show the full graph snapshot
    Graph,
    /// Dynamically inspect a topic's messages
    Echo {
        topic: String,
        #[arg(long)]
        count: Option<usize>,
        #[arg(long)]
        timeout: Option<f64>,
    },
    /// Show metadata for a topic, service, or node
    Info {
        #[arg(value_enum)]
        target: InfoTarget,
        name: String,
    },
    /// Resolve and render a node-local type schema
    Schema {
        type_name: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        schema_hash: String,
    },
    /// Remote parameter operations
    Parameter {
        #[command(subcommand)]
        command: ParameterCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ParameterCommand {
    /// Fetch the full effective parameter snapshot for a node
    Snapshot {
        #[arg(long)]
        node: String,
    },
    /// Fetch one effective parameter value by path
    Get {
        path: String,
        #[arg(long)]
        node: String,
    },
    /// Set one JSON value at a parameter path
    Set {
        path: String,
        value: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        layer: String,
        #[arg(long)]
        expected_revision: Option<u64>,
    },
    /// Reset one layer-local override
    Reset {
        path: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        layer: String,
        #[arg(long)]
        expected_revision: Option<u64>,
    },
    /// Reload parameter overlays from disk
    Reload {
        #[arg(long)]
        node: String,
    },
    /// Watch parameter change events for a node
    Watch {
        #[arg(long)]
        node: String,
    },
}

#[cfg(test)]
mod tests {
    use clap::{Parser, error::ErrorKind};

    use super::{Cli, Command, ListTarget, ParameterCommand};

    #[test]
    fn parses_echo_command_with_defaults() {
        let cli = Cli::parse_from(["rosz", "echo", "/chatter", "--count", "1"]);

        assert_eq!(cli.router, "tcp/127.0.0.1:7447");
        assert!(!cli.json);

        match cli.command {
            Command::Echo {
                topic,
                count,
                timeout,
            } => {
                assert_eq!(topic, "/chatter");
                assert_eq!(count, Some(1));
                assert_eq!(timeout, None);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_global_flags_after_subcommand() {
        let cli = Cli::parse_from([
            "rosz",
            "list",
            "topics",
            "--router",
            "tcp/192.168.1.10:7447",
            "--json",
        ]);

        assert_eq!(cli.router, "tcp/192.168.1.10:7447");
        assert!(cli.json);

        match cli.command {
            Command::List { target } => assert_eq!(target, ListTarget::Topics),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn rejects_domain_flag() {
        let error = Cli::try_parse_from(["rosz", "--domain", "7", "list", "topics"])
            .expect_err("removed flag should be rejected");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_config_set_with_layer_and_revision() {
        let cli = Cli::parse_from([
            "rosz",
            "parameter",
            "set",
            "threshold",
            "0.72",
            "--node",
            "/vision/ball_detector",
            "--layer",
            "./parameter/robot/alpha",
            "--expected-revision",
            "4",
        ]);

        match cli.command {
            Command::Parameter { command } => match command {
                ParameterCommand::Set {
                    path,
                    value,
                    node,
                    layer,
                    expected_revision,
                } => {
                    assert_eq!(path, "threshold");
                    assert_eq!(value, "0.72");
                    assert_eq!(node, "/vision/ball_detector");
                    assert_eq!(layer, "./parameter/robot/alpha");
                    assert_eq!(expected_revision, Some(4));
                }
                other => panic!("unexpected parameter command: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_schema_command_with_required_node() {
        let cli = Cli::parse_from([
            "rosz",
            "schema",
            "hulk_parameters::ObjectDetectionParameters",
            "--node",
            "/vision/object_detection",
            "--schema-hash",
            "RZHS01_deadbeef",
        ]);

        match cli.command {
            Command::Schema {
                type_name,
                node,
                schema_hash,
            } => {
                assert_eq!(type_name, "hulk_parameters::ObjectDetectionParameters");
                assert_eq!(node, "/vision/object_detection");
                assert_eq!(schema_hash, "RZHS01_deadbeef");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn schema_command_requires_node_flag() {
        let error = Cli::try_parse_from([
            "rosz",
            "schema",
            "hulk_parameters::ObjectDetectionParameters",
        ])
        .expect_err("schema command must require --node");

        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn schema_command_requires_schema_hash_flag() {
        let error = Cli::try_parse_from([
            "rosz",
            "schema",
            "hulk_parameters::ObjectDetectionParameters",
            "--node",
            "/vision/object_detection",
        ])
        .expect_err("schema command must require --schema-hash");

        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn parses_parameter_watch_command() {
        let cli = Cli::parse_from([
            "rosz",
            "parameter",
            "watch",
            "--node",
            "/motion/walk_publisher",
        ]);

        match cli.command {
            Command::Parameter { command } => match command {
                ParameterCommand::Watch { node } => {
                    assert_eq!(node, "/motion/walk_publisher");
                }
                other => panic!("unexpected parameter command: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
