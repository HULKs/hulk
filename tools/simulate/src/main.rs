use std::{path::PathBuf, time::Duration};

use bevy::{
    asset::AssetPlugin,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::FrameTimeDiagnosticsPlugin,
    picking::mesh_picking::{MeshPickingCamera, MeshPickingPlugin, MeshPickingSettings},
    prelude::*,
};
use clap::Parser;
use color_eyre::{Result, eyre::Context as _};
use ros_z::prelude::*;
use ros_z::time::{Clock, Time as RosTime};

use crate::{
    bevy_mujoco::MujocoWorldPlugin,
    parameters::{SimulatorParameters, SimulatorParametersPlugin},
    scene::{
        ObjectsPlugin,
        field::FieldPlugin,
        palette::{ObjectPalettePlugin, WorldCamera},
    },
};

mod ball_perception;
mod ball_tuning;
mod behavior_inputs;
mod bevy_mujoco;
mod controls;
mod geometry_inputs;
mod motion_parameters;
mod parameters;
mod remote_ball_tuning;
mod robot_io;
mod robotics;
mod scene;
mod simulated_sdk;
mod simulation;

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, value_name = "DIRECTORY")]
    parameter_root: Option<PathBuf>,
    #[arg(long)]
    router: Option<String>,
    /// Robotics parameter root containing base/, location/, and robot/ layers.
    #[arg(long, default_value = "etc/parameters")]
    robotics_parameter_root: PathBuf,
    /// Additional robotics parameter layers, applied after location and robot layers.
    #[arg(long, value_name = "DIRECTORY", conflicts_with_all = ["tune_ball_filter", "capture_ball_tuning", "remote_ball_tuning"])]
    robotics_parameter_layer: Vec<PathBuf>,
    #[arg(long, default_value = "simulator")]
    location: String,
    #[arg(long)]
    robot: Option<String>,
    #[arg(long, default_value = "/simulator/robot")]
    robot_namespace: String,
    /// Publish sensors and accept raw joint commands without launching robotics nodes.
    #[arg(long)]
    no_robotics: bool,
    /// Run production kinematics, ground/odometry and ball filtering on synthetic detections.
    #[arg(long)]
    ball_perception: bool,
    /// Record headless physical scenarios, optimize on training runs and evaluate holdouts.
    #[arg(long, value_name = "NEW_DIRECTORY", conflicts_with_all = ["no_robotics", "router", "robot", "parameter_root"])]
    tune_ball_filter: Option<PathBuf>,
    /// Capture the six labelled ROS-Z recordings without running parameter search.
    #[arg(long, value_name = "NEW_DIRECTORY", conflicts_with_all = ["tune_ball_filter", "remote_ball_tuning", "no_robotics", "router", "robot", "parameter_root", "ball_perception"])]
    capture_ball_tuning: Option<PathBuf>,
    /// Effective ball-filter parameters for a fresh recording generation.
    #[arg(long, requires = "capture_ball_tuning")]
    capture_ball_parameters: Option<PathBuf>,
    #[arg(long, default_value_t = 0, requires = "capture_ball_tuning")]
    capture_ball_seed_offset: u64,
    /// Opponents in tuning recordings or the live tuning preview.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(0..=8))]
    tuning_opponents: u32,
    /// Opponent cylinder diameter in metres (0.1..1.2).
    #[arg(long, default_value_t = 0.44)]
    tuning_opponent_width: f32,
    /// Multiply behavior walking speeds (0.1..3). Current policy still limits forward to 2 m/s, backward/lateral to 1 m/s.
    #[arg(long, default_value_t = 1.0)]
    tuning_walking_speed_scale: f32,
    /// Monitor a remote bridge snapshot and preview its verified best parameters locally.
    #[arg(long, value_name = "SNAPSHOT_JSON", requires = "remote_tuning_output", conflicts_with_all = ["tune_ball_filter", "no_robotics", "router", "robot", "parameter_root", "ball_perception"])]
    remote_ball_tuning: Option<PathBuf>,
    #[arg(long, value_name = "NEW_DIRECTORY", requires = "remote_ball_tuning")]
    remote_tuning_output: Option<PathBuf>,
    #[arg(long, default_value_t = 4096, requires = "tune_ball_filter")]
    tuning_trials: usize,
    /// Number of balls in the unscored live preview; optimization captures always use one.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=3), requires = "tune_ball_filter")]
    tuning_preview_balls: u8,
    /// Reuse a completed capture directory; warm-start from its saved best if available.
    #[arg(long, value_name = "DIRECTORY", requires = "tune_ball_filter")]
    tuning_recordings: Option<PathBuf>,
    /// Finish after one search round instead of optimizing continuously.
    #[arg(long, requires = "tune_ball_filter")]
    tuning_once: bool,
    /// Keep the Twix progress connection available after optimization finishes.
    #[arg(long, requires = "tune_ball_filter")]
    keep_tuning_open: bool,
    /// Open a read-only 3D view of the local optimizer's live sensor messages.
    #[arg(long, conflicts_with_all = ["tune_ball_filter", "capture_ball_tuning", "remote_ball_tuning", "router", "no_robotics", "ball_perception"])]
    watch_ball_tuning: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let opponents = types::ball_filter_tuning::OpponentParameters {
        count: args.tuning_opponents,
        width: args.tuning_opponent_width,
    };
    color_eyre::eyre::ensure!(opponents.is_valid(), "invalid opponent count or width");
    color_eyre::eyre::ensure!(
        types::ball_filter_tuning::walking_speed_scale_is_valid(args.tuning_walking_speed_scale),
        "walking speed scale must be finite and between 0.1 and 3"
    );
    if args.watch_ball_tuning {
        return scene::tuning_viewer::run();
    }
    if let Some(output) = args.capture_ball_tuning {
        return ball_tuning::run(
            &output,
            1,
            &args.robotics_parameter_root,
            &args.location,
            false,
            ball_tuning::TuningSource::RecordOnly {
                parameters: args.capture_ball_parameters.as_deref(),
                seed_offset: args.capture_ball_seed_offset,
            },
            true,
            1,
            opponents,
            args.tuning_walking_speed_scale,
        );
    }
    if let (Some(snapshot), Some(output)) = (&args.remote_ball_tuning, &args.remote_tuning_output) {
        return ball_tuning::run(
            output,
            1,
            &args.robotics_parameter_root,
            &args.location,
            false,
            ball_tuning::TuningSource::Remote(snapshot),
            false,
            1,
            opponents,
            args.tuning_walking_speed_scale,
        );
    }
    if let Some(output) = args.tune_ball_filter {
        return ball_tuning::run(
            &output,
            args.tuning_trials,
            &args.robotics_parameter_root,
            &args.location,
            args.keep_tuning_open,
            args.tuning_recordings.as_deref().map_or(
                ball_tuning::TuningSource::Record,
                ball_tuning::TuningSource::Recordings,
            ),
            args.tuning_once,
            usize::from(args.tuning_preview_balls),
            opponents,
            args.tuning_walking_speed_scale,
        );
    }
    let parameter_root = args
        .parameter_root
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parameters"));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to build Tokio runtime")?;
    let router = args
        .router
        .clone()
        .unwrap_or_else(|| "tcp/127.0.0.1:7447".into());
    let (context, node, simulator_parameters) = runtime.block_on(async {
        let mut builder = ContextBuilder::default()
            .with_namespace("/simulator")
            .with_parameter_layers([parameter_root.clone()]);
        builder = match args.router {
            Some(router) => builder.with_mode("client").with_router_endpoint(router)?,
            None => builder
                .with_mode("router")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints(["tcp/127.0.0.1:7447"]),
        };

        let context = builder.build().await?;
        let node = context.create_node("parameters").build().await?;
        let simulator_parameters = node.bind_parameter_as::<SimulatorParameters>("simulator")?;
        simulator_parameters.add_validation_hook(SimulatorParameters::validate)?;

        Ok::<_, color_eyre::Report>((context, node, simulator_parameters))
    })?;

    let mut parameter_layers = vec![parameter_root, args.robotics_parameter_root.join("base")];
    parameter_layers.push(
        args.robotics_parameter_root
            .join("location")
            .join(args.location),
    );
    if let Some(robot) = args.robot {
        parameter_layers.push(args.robotics_parameter_root.join("robot").join(robot));
    }
    parameter_layers.extend(args.robotics_parameter_layer);
    let robotics = runtime.block_on(robotics::Robotics::new(
        runtime.handle().clone(),
        robotics::StackConfiguration {
            router,
            namespace: args.robot_namespace,
            parameter_layers,
            launch_nodes: !args.no_robotics,
            ball_perception: args.ball_perception,
        },
        Clock::logical(RosTime::zero()),
    ))?;

    let mut app = App::new();
    app.insert_resource(robotics);
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Motion simulator".into(),
                    resolution: (1600, 900).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            }),
    )
    .add_plugins(MeshPickingPlugin)
    .add_plugins(SimulatorParametersPlugin::new(simulator_parameters.clone()))
    .insert_resource(MeshPickingSettings {
        require_markers: true,
        ..default()
    })
    .add_plugins((
        MujocoWorldPlugin,
        FieldPlugin,
        ObjectsPlugin,
        ObjectPalettePlugin,
        FreeCameraPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        simulation::MotionSimulationPlugin,
        controls::ControlsPlugin,
    ))
    .add_systems(Startup, setup_scene);
    app.run();

    drop(app);
    let shutdown_result = context
        .shutdown()
        .wrap_err("failed to shut down ROS-Z context");
    drop(simulator_parameters);
    drop(node);
    drop(context);
    runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);
    shutdown_result
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        WorldCamera,
        MeshPickingCamera,
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera {
            walk_speed: 3.0,
            run_speed: 10.0,
            ..Default::default()
        },
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn remote_preview_and_capture_are_separate_from_local_search() {
        let refreshed = Args::try_parse_from([
            "simulate",
            "--capture-ball-tuning",
            "capture",
            "--capture-ball-parameters",
            "best.json5",
            "--capture-ball-seed-offset",
            "10000",
            "--tuning-opponents",
            "0",
            "--tuning-opponent-width",
            "0.7",
            "--tuning-walking-speed-scale",
            "2.0",
        ])
        .unwrap();
        assert_eq!(refreshed.capture_ball_seed_offset, 10000);
        assert_eq!(refreshed.tuning_opponents, 0);
        assert_eq!(refreshed.tuning_walking_speed_scale, 2.0);
        assert_eq!(
            refreshed.capture_ball_parameters,
            Some(PathBuf::from("best.json5"))
        );
        assert!(
            Args::try_parse_from(["simulate", "--capture-ball-parameters", "best.json5"]).is_err()
        );
        let capture =
            Args::try_parse_from(["simulate", "--capture-ball-tuning", "capture"]).unwrap();
        assert!(capture.tune_ball_filter.is_none());
        assert_eq!(capture.tuning_walking_speed_scale, 1.0);
        assert!(capture.remote_ball_tuning.is_none());
        let remote = Args::try_parse_from([
            "simulate",
            "--remote-ball-tuning",
            "snapshot.json",
            "--remote-tuning-output",
            "preview",
        ])
        .unwrap();
        assert!(remote.tune_ball_filter.is_none());
        assert!(remote.capture_ball_tuning.is_none());
        assert!(
            Args::try_parse_from(["simulate", "--remote-ball-tuning", "snapshot.json",]).is_err()
        );
        assert!(
            Args::try_parse_from([
                "simulate",
                "--capture-ball-tuning",
                "capture",
                "--tune-ball-filter",
                "search",
            ])
            .is_err()
        );
        assert!(
            Args::try_parse_from([
                "simulate",
                "--capture-ball-tuning",
                "capture",
                "--watch-ball-tuning",
            ])
            .is_err()
        );
    }
}
