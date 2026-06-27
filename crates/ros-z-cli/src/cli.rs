use std::{num::NonZeroUsize, time::Duration};

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

fn parse_positive_nonzero_usize(value: &str) -> Result<NonZeroUsize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|error| format!("invalid positive integer '{value}': {error}"))?;
    NonZeroUsize::new(parsed).ok_or_else(|| "value must be greater than zero".to_string())
}

fn parse_positive_duration(value: &str) -> Result<Duration, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("invalid positive duration '{value}': {error}"))?;
    if parsed <= 0.0 || !parsed.is_finite() {
        return Err("duration must be finite and greater than zero".to_string());
    }
    let duration = Duration::try_from_secs_f64(parsed)
        .map_err(|error| format!("invalid duration '{value}': {error}"))?;
    if duration.is_zero() {
        return Err("duration is too small to represent".to_string());
    }
    Ok(duration)
}

/// Graph entity kind accepted by `rosz list`.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ListTarget {
    /// List topics discovered in the graph.
    Topics,
    /// List nodes discovered in the graph.
    Nodes,
    /// List services discovered in the graph.
    Services,
}

/// Entity kind accepted by `rosz info`.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum InfoTarget {
    /// Show metadata for one topic.
    Topic,
    /// Show metadata for one service.
    Service,
    /// Show metadata for one node.
    Node,
}

/// Parsed top-level CLI arguments for the `rosz` binary.
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

/// Top-level `rosz` command.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate shell completion script
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },

    #[command(flatten)]
    Online(OnlineCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HzLimit {
    Continuous,
    Count(NonZeroUsize),
    Duration(Duration),
}

#[derive(Debug, Args)]
pub struct HzArgs {
    /// Topic to measure.
    pub topic: String,
    /// Number of recent intervals used for rolling statistics.
    #[arg(long, default_value = "10", value_parser = parse_positive_nonzero_usize)]
    pub window: NonZeroUsize,
    /// Stop after receiving this many samples.
    #[arg(long, conflicts_with = "duration", value_parser = parse_positive_nonzero_usize)]
    count: Option<NonZeroUsize>,
    /// Stop after this many seconds.
    #[arg(long, conflicts_with = "count", value_parser = parse_positive_duration)]
    duration: Option<Duration>,
}

impl HzArgs {
    pub fn limit(&self) -> HzLimit {
        match (self.count, self.duration) {
            (Some(count), None) => HzLimit::Count(count),
            (None, Some(duration)) => HzLimit::Duration(duration),
            (None, None) => HzLimit::Continuous,
            (Some(_), Some(_)) => unreachable!("clap rejects count and duration together"),
        }
    }
}

/// Top-level commands that operate on a ros-z graph.
#[derive(Debug, Subcommand)]
pub enum OnlineCommand {
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
    /// Estimate topic message frequency
    Hz(HzArgs),
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

/// Subcommands under `rosz parameter`.
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
    use std::{num::NonZeroUsize, time::Duration};

    use clap::{Parser, error::ErrorKind};

    use super::{Cli, Command, HzLimit, ListTarget, OnlineCommand, ParameterCommand};

    #[test]
    fn parses_echo_command_with_defaults() {
        let cli = Cli::parse_from(["rosz", "echo", "/chatter", "--count", "1"]);

        assert_eq!(cli.router, "tcp/127.0.0.1:7447");
        assert!(!cli.json);

        match cli.command {
            Command::Online(OnlineCommand::Echo {
                topic,
                count,
                timeout,
            }) => {
                assert_eq!(topic, "/chatter");
                assert_eq!(count, Some(1));
                assert_eq!(timeout, None);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_hz_command_with_default_window() {
        let cli = Cli::parse_from(["rosz", "hz", "/chatter"]);

        match cli.command {
            Command::Online(OnlineCommand::Hz(args)) => {
                assert_eq!(args.topic, "/chatter");
                assert_eq!(args.window, NonZeroUsize::new(10).unwrap());
                assert_eq!(args.limit(), HzLimit::Continuous);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_hz_command_with_window_and_count() {
        let cli = Cli::parse_from(["rosz", "hz", "/chatter", "--window", "20", "--count", "5"]);

        match cli.command {
            Command::Online(OnlineCommand::Hz(args)) => {
                assert_eq!(args.topic, "/chatter");
                assert_eq!(args.window, NonZeroUsize::new(20).unwrap());
                assert_eq!(args.limit(), HzLimit::Count(NonZeroUsize::new(5).unwrap()));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_hz_duration_as_duration() {
        let cli = Cli::parse_from(["rosz", "hz", "/chatter", "--duration", "1.5"]);

        match cli.command {
            Command::Online(OnlineCommand::Hz(args)) => {
                assert_eq!(args.limit(), HzLimit::Duration(Duration::from_millis(1500)));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn rejects_hz_duration_overflow() {
        let error = Cli::try_parse_from(["rosz", "hz", "/chatter", "--duration", "1e300"])
            .expect_err("oversized duration should fail validation");

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_hz_count_duration_conflict() {
        let error = Cli::try_parse_from([
            "rosz",
            "hz",
            "/chatter",
            "--count",
            "5",
            "--duration",
            "1.0",
        ])
        .expect_err("count and duration should conflict");

        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn rejects_hz_zero_window() {
        let error = Cli::try_parse_from(["rosz", "hz", "/chatter", "--window", "0"])
            .expect_err("zero window should fail");

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_hz_zero_count() {
        let error = Cli::try_parse_from(["rosz", "hz", "/chatter", "--count", "0"])
            .expect_err("zero count should fail");

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_hz_non_positive_duration() {
        let error = Cli::try_parse_from(["rosz", "hz", "/chatter", "--duration", "0"])
            .expect_err("zero duration should fail");

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_hz_duration_that_rounds_to_zero() {
        let error = Cli::try_parse_from(["rosz", "hz", "/chatter", "--duration", "1e-12"])
            .expect_err("duration rounding to zero should fail validation");

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
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
            Command::Online(OnlineCommand::List { target }) => {
                assert_eq!(target, ListTarget::Topics)
            }
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
            Command::Online(OnlineCommand::Parameter { command }) => match command {
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
            "RZHS02_deadbeef",
        ]);

        match cli.command {
            Command::Online(OnlineCommand::Schema {
                type_name,
                node,
                schema_hash,
            }) => {
                assert_eq!(type_name, "hulk_parameters::ObjectDetectionParameters");
                assert_eq!(node, "/vision/object_detection");
                assert_eq!(schema_hash, "RZHS02_deadbeef");
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
            Command::Online(OnlineCommand::Parameter { command }) => match command {
                ParameterCommand::Watch { node } => {
                    assert_eq!(node, "/motion/walk_publisher");
                }
                other => panic!("unexpected parameter command: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
