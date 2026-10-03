//! Automated physical recording. Captures original ros-z traffic with the robot recorder.
use crate::{
    bevy_mujoco::{MjcfObject, MujocoWorld, MujocoWorldPlugin, SimulationMode},
    parameters::SimulatorParameters,
    remote_ball_tuning::RemoteSnapshot,
    robot_io::RobotBinding,
    robotics::{Robotics, StackConfiguration},
    scene::ball::{SpawnedBalls, ball_spec, first_pose, first_velocity},
};
use bevy::prelude::*;
use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use linear_algebra::Point3;
use projection::Projection;
use ros_z::{
    parameter::{NodeParameterWriteJson, RemoteParameterClient},
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::{Clock, Time as RosTime},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use types::{
    ball_filter_tuning::{
        NAMESPACE, OPEN_VIEWER_TOPIC, OPPONENTS_TOPIC, OpponentParameters, PROGRESS_TOPIC,
        Progress, ROUTER,
    },
    field_dimensions::GlobalFieldSide,
    filtered_game_state::FilteredGameState,
    motion_command::MotionCommand,
    parameters::BallFilterParameters,
};

const EPISODE_SECONDS: f64 = 40.0;

const TOPICS: &[&str] = &[
    "detected_objects",
    "detected_objects/announce",
    "inputs/odometry",
    "inputs/odometry/announce",
    "camera_matrix",
    "field_dimensions",
    "ball_filter/update_schedule",
    "ball_filter/field_prior_pose",
    "ball_filter/obstacles",
    "ball_filter/ball_position",
    "ball_filter/ball_filter_state",
    "ball_filter/ball_percepts",
    "visual_kick/ball_position",
    "ball_state",
    "simulation/ball_ground_truth",
    "simulation/ball_ground_truth_field",
    "simulation/false_ball_detections",
    "simulation/ball_poses_world",
    "simulation/ball_velocities_world",
    "simulation/obstacle_positions_world",
    "obstacles",
    "inputs/serial_motor_states",
    "inputs/imu_state",
    "inputs/camera_info",
    "ground_to_robot",
    "ground_to_field",
    "localization/pose_3d",
    "support_foot_state",
    "simulation/scenario",
    "simulation/parameters",
    "filtered_game_controller_state",
    "primary_state",
    "behavior/motion_command",
];

/// Cargo can replace the on-disk executable while an optimizer keeps running.
/// On Linux current_exe then ends in " (deleted)" and is not executable by path;
/// procfs still exposes the running image, including across a rebuild.
fn viewer_executable(executable: PathBuf) -> PathBuf {
    #[cfg(target_os = "linux")]
    if !executable.is_file() {
        return PathBuf::from("/proc/self/exe");
    }
    executable
}

fn launch_viewer(log_path: &Path) -> std::io::Result<std::process::Child> {
    let executable = viewer_executable(std::env::current_exe()?);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    std::process::Command::new(executable)
        .arg("--watch-ball-tuning")
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()
}

#[derive(Clone, Copy)]
pub enum TuningSource<'a> {
    Record,
    RecordOnly {
        parameters: Option<&'a Path>,
        seed_offset: u64,
    },
    Recordings(&'a Path),
    Remote(&'a Path),
}

pub fn run(
    output: &Path,
    trials: usize,
    parameter_root: &Path,
    location: &str,
    keep_open: bool,
    source: TuningSource<'_>,
    once: bool,
    preview_balls: usize,
    opponents: OpponentParameters,
) -> Result<()> {
    ensure!(
        !output.exists(),
        "output directory already exists: {}",
        output.display()
    );
    ensure!(trials > 0, "tuning trials must be positive");
    ensure!(opponents.is_valid(), "invalid opponent count or width");
    let (capture_parameters, seed_offset) = match source {
        TuningSource::RecordOnly {
            parameters,
            seed_offset,
        } => (
            parameters.map(capture_parameter_override).transpose()?,
            seed_offset,
        ),
        _ => (None, 0),
    };
    ensure!(
        seed_offset <= u64::MAX - 4250,
        "capture seed offset is too large"
    );
    ensure!(
        (1..=3).contains(&preview_balls),
        "preview ball count must be 1 to 3"
    );
    let recordings = match source {
        TuningSource::Recordings(path) => Some(path),
        _ => None,
    };
    if let Some(recordings) = recordings {
        for name in [
            "baseline.json5",
            "train-42.mcap",
            "train-43.mcap",
            "train-142.mcap",
            "train-143.mcap",
            "validation-4243.mcap",
            "validation-4250.mcap",
        ] {
            ensure!(
                recordings.join(name).is_file(),
                "missing recording input: {}",
                recordings.join(name).display()
            );
        }
    }
    std::fs::create_dir_all(output)?;
    write_checkpoint(
        &output.join("scenario.json"),
        &serde_json::to_vec_pretty(&opponents)?,
    )?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = tokio::runtime::Runtime::new()?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let cancellation = shutdown.clone();
    let shutdown_task = runtime.spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancellation.store(true, Ordering::Relaxed);
        }
    });
    let (server, monitor_task, progress, parameter_node) = runtime.block_on(async {
        let server = ContextBuilder::default()
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints([ROUTER])
            .build()
            .await?;
        let node = Arc::new(server
            .create_node("ball_filter_optimizer")
            .with_namespace(NAMESPACE)
            .build()
            .await?);
        let publisher = node
            .publisher::<Progress>(PROGRESS_TOPIC)
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await?;
        let open_viewer = node.subscriber::<bool>(OPEN_VIEWER_TOPIC).build().await?;
        let opponent_requests = node.subscriber::<OpponentParameters>(OPPONENTS_TOPIC).build().await?;
        let (progress, mut updates) = tokio::sync::watch::channel(Progress {
            status: if matches!(source, TuningSource::Remote(_)) {
                "Waiting for remote optimizer"
            } else {
                "Preparing recordings"
            }.into(),
            output_directory: output.display().to_string(),
            recordings: 6,
            duration_seconds: EPISODE_SECONDS,
            opponents,
            ..Default::default()
        });
        let viewer_updates = progress.clone();
        let viewer_log = output.join("3d-viewer.log");
        let scenario_path = output.join("scenario.json");
        let parameter_node = node.clone();
        let task = tokio::spawn(async move {
            let _node = node;
            let mut viewer: Option<std::process::Child> = None;
            let mut heartbeat = tokio::time::interval(Duration::from_millis(500));
            loop {
                tokio::select! {
                    request = opponent_requests.recv() => {
                        if let Ok(request) = request {
                            if request.is_valid() {
                                let saved = serde_json::to_vec_pretty(&request)
                                    .map_err(color_eyre::Report::from)
                                    .and_then(|bytes| write_checkpoint(&scenario_path, &bytes));
                                viewer_updates.send_modify(|state| {
                                    if let Err(error) = saved {
                                        state.error = Some(format!("Cannot save opponent settings: {error:#}"));
                                    } else {
                                        state.opponents = request;
                                    }
                                });
                            }
                        }
                    },
                    _ = heartbeat.tick() => {
                        if let Some(child) = &mut viewer {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    let message = if status.success() {
                                        "3D viewer closed".to_string()
                                    } else {
                                        format!("3D viewer failed ({status}); see {}", viewer_log.display())
                                    };
                                    viewer_updates.send_modify(|state| state.viewer_status = Some(message));
                                    viewer = None;
                                }
                                Err(error) => {
                                    viewer_updates.send_modify(|state| state.viewer_status = Some(format!("3D viewer status: {error}")));
                                    viewer = None;
                                }
                                Ok(None) => {}
                            }
                        }
                    },
                    request = open_viewer.recv() => {
                        if matches!(request, Ok(true)) && viewer.is_none() {
                            let launched = launch_viewer(&viewer_log);
                            let status = match launched {
                                Ok(child) => { viewer = Some(child); "3D viewer process started".to_string() },
                                Err(error) => format!("Could not open 3D viewer: {error}; log: {}", viewer_log.display()),
                            };
                            viewer_updates.send_modify(|state| state.viewer_status = Some(status));
                        }
                    },
                    result = updates.changed() => { if result.is_err() { break; } }
                }
                let state = updates.borrow_and_update().clone();
                if let Err(error) = publisher.publish(&state).await {
                    eprintln!("Could not publish tuning progress: {error:#}");
                }
            }
        });
        Ok::<_, color_eyre::Report>((server, task, progress, parameter_node))
    })?;
    eprintln!(
        "Twix: open Ball-filter optimization and click Connect to simulator / optimizer ({ROUTER})."
    );
    let mut preview = None;
    let result = (|| -> Result<()> {
        let training_seeds = [42, 43, 142, 143];
        let validation_seeds = [4243, 4250];
        let recordings_root = recordings.unwrap_or(output);
        let paths = |kind: &str, seeds: &[u64]| -> Vec<PathBuf> {
            seeds
                .iter()
                .map(|seed| recordings_root.join(format!("{kind}-{seed}.mcap")))
                .collect()
        };
        if matches!(
            source,
            TuningSource::Record | TuningSource::RecordOnly { .. }
        ) {
            for (index, (name, seed)) in training_seeds
                .iter()
                .map(|seed| ("train", *seed))
                .chain(validation_seeds.iter().map(|seed| ("validation", *seed)))
                .enumerate()
            {
                ensure!(!shutdown.load(Ordering::Relaxed), "tuning stopped");
                progress.send_modify(|state| {
                    state.status = "Recording".into();
                    state.recording = format!("{name}-{seed}.mcap");
                    state.recording_index = index as u64 + 1;
                    state.elapsed_seconds = 0.0;
                    state.phase = "Starting robotics stack".into();
                });
                record(
                    runtime.handle(),
                    &root,
                    parameter_root,
                    location,
                    &output.join(format!("{name}-{seed}.mcap")),
                    seed + seed_offset,
                    1, // One unambiguous reference ball in every optimization recording.
                    RosTime::from_nanos(index as i64 * 41_000_000_000),
                    &progress,
                    capture_parameters.as_ref(),
                    opponents,
                    None,
                    &shutdown,
                )?;
            }
        }
        if matches!(source, TuningSource::RecordOnly { .. }) {
            return Ok(());
        }
        // Parameter binding includes all robotics layers. Persist the actual effective
        // baseline alongside results rather than assuming the base layer is complete.
        progress.send_modify(|state| {
            state.status = if matches!(source, TuningSource::Remote(_)) {
                "Waiting for remote optimizer"
            } else {
                "Verifying recorded replay"
            }
            .into();
        });
        let initial_path = recordings_root.join("optimized/ball_filter.json5");
        let mut initial_parameters = initial_path.is_file().then_some(initial_path);
        let baseline_path = recordings_root.join("baseline.json5");
        // Only verified search results are sent to the live production node. The
        // immutable capture set retains its original parameters for exact replay.
        let (best, _) = tokio::sync::watch::channel(None);
        preview = Some(LivePreview::start(
            runtime.handle().clone(),
            root.clone(),
            parameter_root.to_owned(),
            location.to_owned(),
            progress.clone(),
            best.clone(),
            RemoteParameterClient::new(parameter_node, format!("{NAMESPACE}/ball_filter"))?,
            shutdown.clone(),
            preview_balls,
        ));
        if let TuningSource::Remote(snapshot) = source {
            return follow_remote(snapshot, output, &progress, &best, &shutdown);
        }
        let mut sent_trial = None;
        let mut round = 0_u64;
        let mut last_best_trial = 0;
        loop {
            ensure!(!shutdown.load(Ordering::Relaxed), "tuning stopped");
            let checkpoint = output.join(format!("round-{:04}", round + 1));
            let offset = round * trials as u64;
            ball_filter_tuner::run_with_progress(
                ball_filter_tuner::Args {
                    train: paths("train", &training_seeds),
                    validation: paths("validation", &validation_seeds),
                    parameters: baseline_path.clone(),
                    initial_parameters: initial_parameters.clone(),
                    namespace: String::new(),
                    reference_topic: "simulation/ball_ground_truth_field".into(),
                    reference_frame: ball_filter_tuner::ReferenceFrame::Field,
                    trials,
                    seed: 7 + round,
                    penalty_metres: 2.0,
                    output: checkpoint.clone(),
                },
                |search| {
                    ensure!(!shutdown.load(Ordering::Relaxed), "tuning stopped");
                    let mut search = search.clone();
                    if search.best_trial > 0 {
                        last_best_trial = offset + search.best_trial;
                    }
                    search.best_trial = last_best_trial;
                    search.trial += offset;
                    search.trials = if once { trials as u64 } else { 0 };
                    if sent_trial != Some(search.best_trial) {
                        write_checkpoint(
                            &output.join("optimized/ball_filter.json5"),
                            &serde_json::to_vec_pretty(&search.best_parameters)?,
                        )?;
                        best.send_replace(Some(LiveCandidate {
                            trial: search.best_trial,
                            description: format!("trial {}", search.best_trial),
                            parameters: search.best_parameters.clone(),
                        }));
                        sent_trial = Some(search.best_trial);
                    }
                    progress.send_modify(|state| {
                        state.status = if once {
                            "Optimizing".into()
                        } else {
                            format!("Optimizing continuously · round {}", round + 1)
                        };
                        state.search = Some(search.clone());
                    });
                    Ok(())
                },
            )?;
            write_checkpoint(
                &output.join("optimized/report.json"),
                &std::fs::read(checkpoint.join("report.json"))?,
            )?;
            initial_parameters = Some(checkpoint.join("ball_filter.json5"));
            if once {
                break;
            }
            round += 1;
        }
        Ok(())
    })();
    let cancelled = shutdown.load(Ordering::Relaxed);
    progress.send_modify(|state| {
        state.status = if cancelled {
            "Stopped"
        } else if result.is_ok() {
            "Complete"
        } else {
            "Failed"
        }
        .into();
        state.error = if cancelled {
            None
        } else {
            result.as_ref().err().map(|error| format!("{error:#}"))
        };
    });
    if let Err(error) = &result
        && !cancelled
    {
        eprintln!("Ball-filter tuning failed: {error:#}");
    }
    if keep_open && !cancelled {
        eprintln!("Keeping live simulation and Twix monitor available. Press Ctrl-C to exit.");
        while !shutdown.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        // Give observers time to receive the final state before closing transport.
        std::thread::sleep(Duration::from_millis(600));
    }
    drop(preview);
    monitor_task.abort();
    shutdown_task.abort();
    server.shutdown()?;
    runtime.shutdown_timeout(Duration::from_secs(2));
    if cancelled { Ok(()) } else { result }
}

fn follow_remote(
    snapshot_path: &Path,
    output: &Path,
    progress: &tokio::sync::watch::Sender<Progress>,
    best: &tokio::sync::watch::Sender<Option<LiveCandidate>>,
    shutdown: &AtomicBool,
) -> Result<()> {
    let mut candidate_id = String::new();
    while !shutdown.load(Ordering::Relaxed) {
        let result = (|| -> Result<()> {
            let snapshot: RemoteSnapshot = serde_json::from_slice(&std::fs::read(snapshot_path)?)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64();
            let problem = snapshot.problem(now);
            if problem.is_none()
                && snapshot.remote.best_candidate != candidate_id
                && let Some(search) = &snapshot.search
            {
                write_checkpoint(
                    &output.join("optimized/ball_filter.json5"),
                    &serde_json::to_vec_pretty(&search.best_parameters)?,
                )?;
                best.send_replace(Some(LiveCandidate {
                    trial: search.best_trial,
                    description: snapshot.remote.best_candidate.clone(),
                    parameters: search.best_parameters.clone(),
                }));
                candidate_id = snapshot.remote.best_candidate.clone();
            }
            progress.send_modify(|state| {
                state.status = if problem.is_some() {
                    "Remote optimizer connection needs attention".into()
                } else {
                    format!(
                        "Remote optimization · {} workers",
                        snapshot.remote.workers.len()
                    )
                };
                state.error = problem;
                state.remote = Some(snapshot.remote);
                state.remote_updated_unix_seconds = Some(snapshot.updated_unix_seconds);
                state.search = snapshot.search;
            });
            Ok(())
        })();
        if let Err(error) = result {
            progress.send_modify(|state| {
                state.status = "Waiting for remote optimizer bridge".into();
                state.error = Some(format!("{}: {error:#}", snapshot_path.display()));
            });
        }
        // Polling and shutdown remain independent of SSH or remote workers.
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

fn write_checkpoint(path: &Path, contents: &[u8]) -> Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| eyre!("checkpoint has no directory"))?;
    std::fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    use std::io::Write;
    file.write_all(contents)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[derive(Clone)]
struct LiveCandidate {
    trial: u64,
    description: String,
    parameters: BallFilterParameters,
}

struct LiveUpdates {
    best: tokio::sync::watch::Receiver<Option<LiveCandidate>>,
    stop: Arc<AtomicBool>,
    client: RemoteParameterClient,
}

struct LivePreview {
    stop: Arc<AtomicBool>,
    task: Option<std::thread::JoinHandle<()>>,
    // Keep the final candidate available after the search has finished.
    _best: tokio::sync::watch::Sender<Option<LiveCandidate>>,
}

impl LivePreview {
    #[allow(clippy::too_many_arguments)]
    fn start(
        runtime: tokio::runtime::Handle,
        root: PathBuf,
        parameter_root: PathBuf,
        location: String,
        progress: tokio::sync::watch::Sender<Progress>,
        best: tokio::sync::watch::Sender<Option<LiveCandidate>>,
        client: RemoteParameterClient,
        stop: Arc<AtomicBool>,
        ball_count: usize,
    ) -> Self {
        let mut updates = LiveUpdates {
            best: best.subscribe(),
            stop: stop.clone(),
            client,
        };
        progress.send_modify(|state| {
            state.live_status = Some("Waiting for verified search parameters".into())
        });
        let task = std::thread::spawn(move || {
            let mut episode = 0_u64;
            while !updates.stop.load(Ordering::Relaxed) {
                if updates.best.borrow().is_none() {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                progress.send_modify(|state| {
                    state.live_trial = None;
                    state.live_status = Some(format!("Starting live episode {}", episode + 1));
                    state.elapsed_seconds = 0.0;
                });
                let stop = updates.stop.clone();
                let opponents = progress.borrow().opponents;
                let result = record(
                    &runtime,
                    &root,
                    &parameter_root,
                    &location,
                    Path::new("live preview"),
                    242 + episode,
                    ball_count,
                    RosTime::from_nanos((6 + episode) as i64 * 41_000_000_000),
                    &progress,
                    None,
                    opponents,
                    Some(&mut updates),
                    &stop,
                );
                if let Err(error) = result
                    && !updates.stop.load(Ordering::Relaxed)
                {
                    eprintln!("Live preview restarting: {error:#}");
                    progress.send_modify(|state| {
                        state.live_trial = None;
                        state.live_status = Some(format!("Live preview restarting: {error:#}"));
                    });
                    // A failed preview does not discard the search or its recordings.
                    for _ in 0..50 {
                        if updates.stop.load(Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
                episode += 1;
            }
        });
        Self {
            stop,
            task: Some(task),
            _best: best,
        }
    }
}

impl Drop for LivePreview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            if task.join().is_err() {
                eprintln!("Live preview thread panicked");
            }
        }
    }
}

async fn apply_live_parameters(
    client: &RemoteParameterClient,
    parameters: &BallFilterParameters,
) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(3), async {
        let snapshot = client.get_snapshot().await?;
        ensure!(
            snapshot.success,
            "cannot read live ball-filter parameters: {}",
            snapshot.message
        );
        let layer = snapshot
            .layers
            .last()
            .ok_or_else(|| eyre!("ball filter has no writable parameter layer"))?;
        let value = serde_json::to_value(parameters)?;
        let writes = value
            .as_object()
            .ok_or_else(|| eyre!("expected ball-filter parameter object"))?
            .iter()
            .map(|(path, value)| {
                Ok(NodeParameterWriteJson {
                    path: path.clone(),
                    value_json: serde_json::to_string(value)?,
                    target_layer: layer.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let response = client
            .set_json_atomically(writes, Some(snapshot.revision))
            .await?;
        ensure!(
            response.success,
            "cannot apply live ball-filter parameters: {}",
            response.message
        );
        Ok::<_, color_eyre::Report>(())
    })
    .await??;
    Ok(())
}

fn capture_parameter_override(path: &Path) -> Result<serde_json::Value> {
    let parameters: serde_json::Value = json5::from_str(&std::fs::read_to_string(path)?)?;
    // Validate a historical best, but preserve its original keys. New captures
    // inherit newly introduced settings from current production layers, while
    // replay of old baselines still uses their legacy deserialization defaults.
    let _: BallFilterParameters = serde_json::from_value(parameters.clone())?;
    Ok(parameters)
}

#[allow(clippy::too_many_arguments)]
fn record(
    runtime: &tokio::runtime::Handle,
    root: &Path,
    parameter_root: &Path,
    location: &str,
    path: &Path,
    seed: u64,
    ball_count: usize,
    start_time: RosTime,
    progress: &tokio::sync::watch::Sender<Progress>,
    capture_parameters: Option<&serde_json::Value>,
    opponents: OpponentParameters,
    mut live: Option<&mut LiveUpdates>,
    stop: &AtomicBool,
) -> Result<()> {
    let layer = tempfile::tempdir()?;
    std::fs::write(
        layer.path().join("motion_inference.json5"),
        serde_json::json!({
            "neural_networks_folder": root.join("etc/neural_networks")
        })
        .to_string(),
    )?;
    let initial = live
        .as_mut()
        .and_then(|updates| updates.best.borrow_and_update().clone());
    let live_parameters = initial
        .as_ref()
        .map(|candidate| serde_json::to_value(&candidate.parameters))
        .transpose()?;
    if let Some(parameters) = live_parameters.as_ref().or(capture_parameters) {
        std::fs::write(
            layer.path().join("ball_filter.json5"),
            serde_json::to_string(parameters)?,
        )?;
    }
    let mut parameters: SimulatorParameters = json5::from_str(&std::fs::read_to_string(
        root.join("tools/simulate/parameters/simulator.json5"),
    )?)?;
    parameters.ball_perception.seed = seed;
    parameters.opponents = opponents;
    if !seed.is_multiple_of(2) {
        let noise = &mut parameters.ball_perception;
        noise.center_noise_pixels = 5.0;
        noise.center_bias_pixels = [3.0, -2.0];
        noise.false_positive_probability = 0.08;
        noise.false_positive_burst_frames = 8;
        noise.dropout_probability = 0.08;
        noise.dropout_burst_probability = 0.03;
        noise.dropout_burst_frames = 8;
    }
    SimulatorParameters::validate(&parameters).map_err(|e| eyre!(e))?;
    let clock = Clock::logical(start_time);
    let (
        mut io,
        recording,
        phase_pub,
        parameters_pub,
        ball_poses_pub,
        ball_velocities_pub,
        obstacles_pub,
    ) = runtime.block_on(async {
        let io = Robotics::new(
            runtime.clone(),
            StackConfiguration {
                router: ROUTER.into(),
                namespace: NAMESPACE.into(),
                parameter_layers: vec![
                    root.join("tools/simulate/parameters"),
                    parameter_root.join("base"),
                    parameter_root.join("location").join(location),
                    layer.path().to_owned(),
                ],
                launch_nodes: true,
                ball_perception: true,
            },
            clock.clone(),
        )
        .await?;
        let phase = io
            .node()
            .publisher::<types::time_wrapper::TimeWrapper<String>>("simulation/scenario")
            .build()
            .await?;
        let config = io
            .node()
            .publisher::<SimulatorParameters>("simulation/parameters")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await?;
        let ball_poses = io
            .node()
            .publisher::<types::time_wrapper::TimeWrapper<Vec<nalgebra::Isometry3<f32>>>>(
                "simulation/ball_poses_world",
            )
            .build()
            .await?;
        let ball_velocities = io
            .node()
            .publisher::<types::time_wrapper::TimeWrapper<Vec<nalgebra::Vector3<f32>>>>(
                "simulation/ball_velocities_world",
            )
            .build()
            .await?;
        let obstacles_pub = io
            .node()
            .publisher::<types::time_wrapper::TimeWrapper<Vec<nalgebra::Point3<f32>>>>(
                "simulation/obstacle_positions_world",
            )
            .build()
            .await?;
        let recording = if live.is_none() {
            let baseline = io
                .node()
                .bind_parameter_as::<types::parameters::BallFilterParameters>("ball_filter")?;
            std::fs::write(
                path.with_file_name("baseline.json5"),
                serde_json::to_string_pretty(baseline.snapshot().typed())?,
            )?;
            Some(mcap_recorder::Recording::start(io.node(), path.to_owned(), TOPICS).await?)
        } else {
            None
        };
        Ok::<_, color_eyre::Report>((
            io,
            recording,
            phase,
            config,
            ball_poses,
            ball_velocities,
            obstacles_pub,
        ))
    })?;
    io.input_game.game_state = FilteredGameState::Playing {
        ball_is_free: true,
        kick_off: false,
    };
    io.input_game.global_field_side = GlobalFieldSide::Home;
    io.clear_injection()?;
    runtime.block_on(parameters_pub.publish(&parameters))?;
    progress.send_modify(|state| state.active_opponents = Some(opponents));
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
    app.insert_resource(SimulationMode::Paused);
    app.world_mut()
        .spawn(crate::scene::walls::object(parameters.field_dimensions));
    let robot = app
        .world_mut()
        .spawn((
            MjcfObject::new(root.join("tools/simulate/assets/k1_robot.xml"), "Trunk")
                .with_free_joint("world_joint")
                .grounded(),
            Transform::default(),
        ))
        .id();
    let variation = ScenarioVariation::new(seed);
    let radius = parameters.field_dimensions.ball_radius;
    let mut balls = vec![spawn_ball(
        &mut app,
        &parameters,
        [
            if seed.is_multiple_of(2) { 3.8 } else { 4.2 },
            0.65 * variation.side,
            0.0,
        ],
    )];
    // Additional balls are an explicit, unscored live-preview stress test.
    ensure!(
        live.is_some() || ball_count == 1,
        "optimization captures require one ball"
    );
    for index in 1..ball_count {
        balls.push(spawn_ball(
            &mut app,
            &parameters,
            if index == 1 {
                [-2.4, 1.4 * variation.side, 0.0]
            } else {
                [1.0, -2.0 * variation.side, 0.0]
            },
        ));
    }
    let dimensions = parameters.field_dimensions;
    let mut challenge = crate::scene::tuning_obstacles::Challenge::with_opponents(
        seed,
        [
            f64::from(dimensions.length / 2.0 + dimensions.border_strip_width),
            f64::from(dimensions.width / 2.0 + dimensions.border_strip_width),
        ],
        f64::from(radius),
        opponents,
    )
    .map_err(|error| eyre!(error))?;
    let initial_balls: Vec<_> = balls
        .iter()
        .map(|entity| {
            let transform = app.world().get::<Transform>(*entity).unwrap();
            [
                f64::from(transform.translation.x),
                -f64::from(transform.translation.z),
            ]
        })
        .collect();
    challenge
        .avoid_initial_balls(&initial_balls)
        .map_err(|error| eyre!(error))?;
    let obstacles: Vec<_> = challenge
        .positions()
        .into_iter()
        .map(|position| {
            app.world_mut()
                .spawn((
                    crate::scene::tuning_obstacles::object(opponents.width / 2.0),
                    crate::scene::tuning_obstacles::transform(position),
                ))
                .id()
        })
        .collect();
    app.update();
    {
        let world = app.world().resource::<MujocoWorld>();
        let binding = RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))?;
        io.publish_field_dimensions(&parameters.field_dimensions)?;
        io.publish_observation(binding.observe(world.data()), start_time)?;
        io.publish_inputs()?;
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while io.physics_blocker().is_some() {
        ensure!(!stop.load(Ordering::Relaxed), "tuning stopped");
        if live
            .as_ref()
            .is_some_and(|updates| updates.stop.load(Ordering::Relaxed))
        {
            return Ok(());
        }
        ensure!(
            std::time::Instant::now() < deadline,
            "robotics startup failed: {}",
            io.status()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    if let Some(candidate) = initial {
        progress.send_modify(|state| {
            state.live_trial = Some(candidate.trial);
            state.live_status = Some(format!(
                "Live simulator using best parameters from {}",
                candidate.description
            ));
        });
    }
    let mut last_phase = usize::MAX;
    let mut walk_distance = 0.0;
    let mut ball_distance = 0.0;
    let mut simultaneous_motion_seconds = 0.0;
    let mut fast_ball_seconds = 0.0;
    let mut peak_ball_speed = 0.0_f64;
    let mut previous_robot: Option<nalgebra::Vector3<f32>> = None;
    let mut previous_balls = vec![None::<nalgebra::Vector3<f64>>; ball_count];
    let mut pending_impulses = Vec::new();
    let mut opponent_kicks = 0_u64;
    let mut occluded_kicks = 0_u64;
    let mut occluded_in_view_kicks = 0_u64;
    let mut contest_seconds = 0.0;
    let mut occluded_in_view_seconds = 0.0;

    for frame in 0..2500 {
        ensure!(!stop.load(Ordering::Relaxed), "tuning stopped");
        if live.is_some() && progress.borrow().opponents != opponents {
            progress.send_modify(|state| {
                state.active_opponents = None;
                state.live_status = Some("Restarting preview to apply opponent settings".into());
            });
            return Ok(());
        }
        if let Some(updates) = live.as_mut() {
            if updates.stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            if updates.best.has_changed().unwrap_or(false) {
                let candidate = updates.best.borrow_and_update().clone();
                if let Some(candidate) = candidate {
                    runtime.block_on(apply_live_parameters(
                        &updates.client,
                        &candidate.parameters,
                    ))?;
                    progress.send_modify(|state| {
                        state.live_trial = Some(candidate.trial);
                        state.live_status = Some(format!(
                            "Live simulator using best parameters from {}",
                            candidate.description
                        ));
                    });
                    eprintln!(
                        "Applied best parameters from {} to live ball filter",
                        candidate.description
                    );
                }
            }
        }
        let seconds = frame as f64 * 0.016;
        let phase = scenario_phase(seconds);
        let step = &SCENARIO[phase];
        if phase != last_phase {
            progress.send_modify(|state| {
                state.phase = format!(
                    "{} / {} noise",
                    step.name,
                    if seed.is_multiple_of(2) {
                        "baseline"
                    } else {
                        "stress"
                    }
                )
            });
            eprintln!("{}: {seconds:.1}s {}", path.display(), step.name);
            runtime.block_on(phase_pub.publish(&types::time_wrapper::TimeWrapper {
                time: clock.now(),
                inner: step.name.into(),
            }))?;
            if !step.ball_present {
                for entity in balls.drain(..) {
                    app.world_mut().despawn(entity);
                }
                app.update();
                previous_balls.fill(None);
            } else if balls.is_empty() {
                for index in 0..ball_count {
                    let position = {
                        let world = app.world().resource::<MujocoWorld>();
                        let binding = RobotBinding::new(
                            world.data(),
                            &format!("object_{}_", robot.to_bits()),
                        )?;
                        let mut p = binding.ground_to_world(world.data())
                            * nalgebra::point![
                                0.6 + index as f32 * 0.6,
                                (1.5 - index as f32 * 1.4) * variation.side as f32,
                                radius
                            ];
                        // A new ball must appear inside the walls even near a field edge.
                        let dimensions = parameters.field_dimensions;
                        let x_bound =
                            dimensions.length / 2.0 + dimensions.border_strip_width - 2.0 * radius;
                        let y_bound =
                            dimensions.width / 2.0 + dimensions.border_strip_width - 2.0 * radius;
                        p.x = p.x.clamp(-x_bound, x_bound);
                        p.y = p.y.clamp(-y_bound, y_bound);
                        [f64::from(p.x), f64::from(p.y), f64::from(p.z)]
                    };
                    balls.push(spawn_ball(&mut app, &parameters, position));
                }
                app.update();
                previous_balls.fill(None);
            }
            if let Some(impulse) = step.ball_impulse {
                let world = app.world().resource::<MujocoWorld>();
                let binding =
                    RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))?;
                for (index, ball) in balls.iter().enumerate() {
                    let rotation = nalgebra::Rotation2::new(index as f32 * 1.2);
                    let planar = rotation
                        * nalgebra::vector![
                            (impulse[0] * variation.speed) as f32,
                            (impulse[1] * variation.side) as f32
                        ];
                    let impulse = binding.ground_to_world(world.data()).rotation
                        * nalgebra::vector![planar.x, planar.y, 0.0,];
                    pending_impulses.push(BallImpulse {
                        ball: *ball,
                        impulse: [f64::from(impulse.x), f64::from(impulse.y)],
                        height_above_center: 0.4 * f64::from(radius),
                    });
                }
            }
            last_phase = phase;
        }
        let mut world = app.world_mut().resource_mut::<MujocoWorld>();
        let binding = RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))?;
        let walking = matches!(
            io.active_motion(),
            MotionCommand::Walk { .. } | MotionCommand::WalkWithVelocity { .. }
        );
        for _ in 0..8 {
            let robot_pose = binding.ground_to_world(world.data());
            let ball_world_positions = balls
                .iter()
                .map(|&ball| first_pose(&world, &SpawnedBalls(vec![ball])).map(|(p, _)| p))
                .collect::<Result<Vec<_>>>()?;
            let challenge_frame = challenge.step(
                0.002,
                [
                    f64::from(robot_pose.translation.x),
                    f64::from(robot_pose.translation.y),
                ],
                &ball_world_positions,
            );
            let obstacle_positions = challenge_frame.positions;
            if challenge_frame.contesting {
                contest_seconds += 0.002;
            }
            let occluders: Vec<_> = obstacle_positions
                .iter()
                .map(|p| crate::ball_perception::Occluder {
                    center: Point3::wrap(binding.point_in_ground(world.data(), *p)),
                    radius: opponents.width / 2.0,
                    height: crate::scene::tuning_obstacles::HEIGHT,
                })
                .collect();
            let camera = binding.observe(world.data()).camera_matrix;
            let mut blocked_in_view = false;
            for (index, &position) in ball_world_positions.iter().enumerate() {
                let ball = Point3::wrap(binding.point_in_ground(world.data(), position));
                let in_view = camera
                    .ground_with_z_to_pixel(ball.xy(), ball.z())
                    .is_ok_and(|pixel| crate::ball_perception::in_image(&camera, pixel));
                let blocked = occluders
                    .iter()
                    .any(|obstacle| crate::ball_perception::occludes(&camera, ball, obstacle));
                blocked_in_view |= blocked && in_view;
                if let Some(kick) = challenge_frame
                    .kick
                    .as_ref()
                    .filter(|kick| kick.ball_index == index)
                {
                    opponent_kicks += 1;
                    occluded_kicks += u64::from(blocked);
                    occluded_in_view_kicks += u64::from(blocked && in_view);
                    pending_impulses.push(BallImpulse {
                        ball: balls[index],
                        impulse: kick.impulse,
                        height_above_center: 0.4 * f64::from(radius),
                    });
                    let event =
                        format!("opponent kick / camera blocked={blocked}, in view={in_view}");
                    eprintln!("{}: {:.3}s {event}", path.display(), world.data().time());
                    progress.send_modify(|state| state.phase = event.clone());
                    runtime.block_on(phase_pub.publish(&types::time_wrapper::TimeWrapper {
                        time: clock.now(),
                        inner: event,
                    }))?;
                }
            }
            if blocked_in_view {
                occluded_in_view_seconds += 0.002;
            }
            for (&entity, &position) in obstacles.iter().zip(&obstacle_positions) {
                world
                    .set_object_pose(entity, crate::scene::tuning_obstacles::transform(position))
                    .map_err(|error| eyre!(error))?;
            }
            binding.apply(world.data_mut(), io.latest_command().as_ref());
            step_with_ball_impulse(&mut world, pending_impulses.pop())?;
            let data = world.data();
            let time = start_time + Duration::from_secs_f64(data.time());
            let observation = binding.observe(data);
            ensure!(
                observation.robot_to_world.translation.z > 0.3,
                "robot fell during {seconds:.1}s; refusing unrepresentative capture"
            );
            let robot_position = observation.robot_to_world.translation.vector;
            let robot_step = previous_robot
                .map(|last| (robot_position.xy() - last.xy()).norm())
                .unwrap_or(0.0);
            previous_robot = Some(robot_position);
            if walking {
                walk_distance += robot_step;
            }
            let mut ball_poses = Vec::new();
            let mut ball_velocities = Vec::new();
            let mut positions = Vec::new();
            let mut any_ball_fast = false;
            let mut any_ball_moving = false;
            for (index, &ball) in balls.iter().enumerate() {
                let (p, q) = first_pose(&world, &SpawnedBalls(vec![ball]))?;
                ball_velocities.push(
                    nalgebra::Vector3::from(first_velocity(&world, &SpawnedBalls(vec![ball]))?)
                        .map(|v| v as f32),
                );
                ball_poses.push(nalgebra::Isometry3::from_parts(
                    nalgebra::Translation3::from(p.map(|v| v as f32)),
                    nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
                        q[0] as f32,
                        q[1] as f32,
                        q[2] as f32,
                        q[3] as f32,
                    )),
                ));
                let position = nalgebra::Vector3::from(p);
                let ball_step = previous_balls[index]
                    .map(|last| (position.xy() - last.xy()).norm())
                    .unwrap_or(0.0);
                ball_distance += ball_step;
                let ball_speed = ball_step / 0.002;
                peak_ball_speed = peak_ball_speed.max(ball_speed);
                any_ball_fast |= ball_speed > 2.0;
                any_ball_moving |= ball_speed > 0.08;
                previous_balls[index] = Some(position);
                positions.push(Point3::wrap(binding.point_in_ground(data, p)));
            }
            if any_ball_fast {
                fast_ball_seconds += 0.002;
            }
            if walking && robot_step / 0.002 > 0.04 && any_ball_moving {
                simultaneous_motion_seconds += 0.002;
            }
            runtime.block_on(ball_poses_pub.publish_with_source_time(
                &types::time_wrapper::TimeWrapper {
                    time,
                    inner: ball_poses,
                },
                time,
            ))?;
            runtime.block_on(
                obstacles_pub.publish_with_source_time(
                    &types::time_wrapper::TimeWrapper {
                        time,
                        inner: obstacle_positions
                            .iter()
                            .map(|p| nalgebra::Point3::from(p.map(|v| v as f32)))
                            .collect(),
                    },
                    time,
                ),
            )?;
            runtime.block_on(ball_velocities_pub.publish_with_source_time(
                &types::time_wrapper::TimeWrapper {
                    time,
                    inner: ball_velocities,
                },
                time,
            ))?;
            let ground = binding.ground_to_world(data);
            io.ball_perception
                .as_mut()
                .ok_or_else(|| eyre!("missing perception"))?
                .publish_with_occluders(
                    time,
                    crate::behavior_inputs::ground_to_field(
                        ground,
                        io.input_game.global_field_side,
                    ),
                    &observation.camera_matrix,
                    positions,
                    radius,
                    &parameters.ball_perception,
                    &obstacle_positions
                        .iter()
                        .map(|p| crate::ball_perception::Occluder {
                            center: Point3::wrap(binding.point_in_ground(data, *p)),
                            radius: opponents.width / 2.0,
                            height: crate::scene::tuning_obstacles::HEIGHT,
                        })
                        .collect::<Vec<_>>(),
                )?;
            io.publish_observation(observation, time)?;
            io.publish_world(
                ground,
                None,
                obstacle_positions.to_vec(),
                [opponents.width / 2.0; 2],
                time,
            )?;
            io.publish_inputs()?;
            // Let the asynchronous sensor and control stack run between physics
            // samples, as on a robot; a headless run need not catch up render frames.
            std::thread::sleep(Duration::from_millis(2));
        }
        if frame % 8 == 0 {
            progress.send_modify(|state| state.elapsed_seconds = seconds + 0.016);
        }
        ensure!(
            io.physics_blocker().is_none(),
            "robotics stopped: {}",
            io.status()
        );
    }
    // Live previews are deliberately separate from the fixed replay dataset.
    let Some(recording) = recording else {
        return Ok(());
    };
    // Autonomous behavior can legitimately search or stop after losing the ball.
    // Keep those difficult filter examples instead of biasing captures toward pursuit.
    if walk_distance <= 0.2 || simultaneous_motion_seconds <= 2.0 {
        eprintln!(
            "Low autonomous motion coverage: walked {walk_distance:.3}m, simultaneous motion {simultaneous_motion_seconds:.2}s; retaining the recording"
        );
    }
    ensure!(
        ball_distance > 0.2,
        "rolling ball only moved {ball_distance:.3}m"
    );
    ensure!(
        peak_ball_speed > 2.5 && fast_ball_seconds > 0.5,
        "fast kicks reached only {peak_ball_speed:.2}m/s with {fast_ball_seconds:.2}s above 2m/s"
    );
    progress.send_modify(|state| state.elapsed_seconds = EPISODE_SECONDS);
    // Allow the fusion safety lag and transport queues to drain without more sensors.
    clock.advance(Duration::from_millis(100))?;
    std::thread::sleep(Duration::from_millis(300));
    let written = runtime.block_on(recording.finish())?;
    let coverage = serde_json::json!({
        "seed": seed,
        "ball_count": ball_count,
        "opponent_kicks": opponent_kicks,
        "occluded_kicks": occluded_kicks,
        "occluded_in_view_kicks": occluded_in_view_kicks,
        "contest_seconds": contest_seconds,
        "occluded_in_view_seconds": occluded_in_view_seconds,
        "robot_walked_metres": walk_distance,
        "ball_travel_metres": ball_distance,
        "simultaneous_motion_seconds": simultaneous_motion_seconds,
        "peak_ball_speed_metres_per_second": peak_ball_speed,
        "fast_ball_seconds": fast_ball_seconds,
    });
    write_checkpoint(
        &path.with_extension("coverage.json"),
        &serde_json::to_vec_pretty(&coverage)?,
    )?;
    eprintln!(
        "Contest coverage: {opponent_kicks} opponent kicks, {occluded_kicks} blocked by opponents ({occluded_in_view_kicks} in camera view); {contest_seconds:.2}s contested, {occluded_in_view_seconds:.2}s occluded in view"
    );
    eprintln!(
        "Recorded {written} messages; robot walked {walk_distance:.3}m, ball rolled {ball_distance:.3}m; simultaneous motion {simultaneous_motion_seconds:.2}s; peak ball speed {peak_ball_speed:.2}m/s, fast ball {fast_ball_seconds:.2}s"
    );
    drop(io);
    Ok(())
}

struct ScenarioStep {
    until: f64,
    name: &'static str,
    ball_present: bool,
    ball_impulse: Option<[f64; 2]>,
}

const SCENARIO: [ScenarioStep; 10] = [
    ScenarioStep {
        until: 3.0,
        name: "playing / stationary ball",
        ball_present: true,
        ball_impulse: None,
    },
    ScenarioStep {
        until: 9.0,
        name: "playing / incoming diagonal ball",
        ball_present: true,
        ball_impulse: Some([-0.11, -0.27]),
    },
    ScenarioStep {
        until: 10.2,
        name: "playing / fast cross-field kick",
        ball_present: true,
        ball_impulse: Some([0.11, 1.53]),
    },
    ScenarioStep {
        until: 14.0,
        name: "playing / fast ball redirected",
        ball_present: true,
        ball_impulse: Some([-0.20, -1.75]),
    },
    ScenarioStep {
        until: 18.0,
        name: "playing / rolling ball nudged",
        ball_present: true,
        ball_impulse: Some([-0.18, 0.07]),
    },
    ScenarioStep {
        until: 24.0,
        name: "playing / ball redirected",
        ball_present: true,
        ball_impulse: Some([0.35, -0.20]),
    },
    ScenarioStep {
        until: 28.0,
        name: "playing / lateral ball impulse",
        ball_present: true,
        ball_impulse: Some([-0.30, 0.50]),
    },
    ScenarioStep {
        until: 34.0,
        name: "playing / empty scene with false detections",
        ball_present: false,
        ball_impulse: None,
    },
    ScenarioStep {
        until: 35.2,
        name: "playing / fast incoming ball reappears",
        ball_present: true,
        ball_impulse: Some([0.0, -1.8]),
    },
    ScenarioStep {
        until: EPISODE_SECONDS,
        name: "playing / reacquiring redirected ball",
        ball_present: true,
        ball_impulse: Some([1.50, 0.90]),
    },
];

fn scenario_phase(seconds: f64) -> usize {
    SCENARIO
        .iter()
        .position(|step| seconds < step.until)
        .unwrap_or(SCENARIO.len() - 1)
}

struct ScenarioVariation {
    speed: f64,
    side: f64,
}
impl ScenarioVariation {
    fn new(seed: u64) -> Self {
        let mixed = seed.wrapping_mul(0x9e3779b97f4a7c15);
        Self {
            speed: 0.85 + 0.30 * ((mixed >> 32) & 255) as f64 / 255.0,
            side: if mixed & (1 << 16) == 0 { 1.0 } else { -1.0 },
        }
    }
}

fn spawn_ball(app: &mut App, parameters: &SimulatorParameters, position: [f64; 3]) -> Entity {
    let physical = parameters.ball.clone();
    let radius = parameters.field_dimensions.ball_radius;
    app.world_mut()
        .spawn((
            MjcfObject::from_factory(move || ball_spec(f64::from(radius), &physical), "ball")
                .with_free_joint("ball_free_joint")
                .grounded(),
            Transform::from_xyz(position[0] as f32, 0.0, -position[1] as f32),
        ))
        .id()
}

/// World-frame impulse (N s) applied at a specified height above the ball's COM.
#[derive(Clone, Copy)]
struct BallImpulse {
    ball: Entity,
    impulse: [f64; 2],
    height_above_center: f64,
}

/// Integrate a finite-duration impulse through MuJoCo's external force API.
/// Never edits positions, velocities or spin. Restore prior forces after one step.
fn step_with_ball_impulse(world: &mut MujocoWorld, impulse: Option<BallImpulse>) -> Result<()> {
    let data = world.data_mut();
    if let Some(impulse) = impulse {
        let dt = data.model().opt().timestep;
        let body = data
            .body(&format!("object_{}_ball", impulse.ball.to_bits()))
            .ok_or_else(|| eyre!("missing ball body"))?;
        let force = [impulse.impulse[0] / dt, impulse.impulse[1] / dt];
        let previous = body.view(data).xfrc_applied.to_vec();
        let applied = [
            force[0],
            force[1],
            0.0,
            -impulse.height_above_center * force[1],
            impulse.height_above_center * force[0],
            0.0,
        ];
        for (target, force) in body.view_mut(data).xfrc_applied.iter_mut().zip(applied) {
            *target += force;
        }
        data.step();
        body.view_mut(data).xfrc_applied.copy_from_slice(&previous);
    } else {
        data.step();
    }
    data.forward();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn replaced_optimizer_executable_uses_the_running_image_for_viewer() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("simulate");
        std::fs::write(&original, b"placeholder executable").unwrap();
        assert_eq!(viewer_executable(original.clone()), original);
        std::fs::remove_file(&original).unwrap();
        let deleted = directory.path().join("simulate (deleted)");
        let fallback = viewer_executable(deleted);
        assert_eq!(fallback, std::path::Path::new("/proc/self/exe"));
        assert!(fallback.is_file(), "running image remains accessible");
    }

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn fresh_capture_inherits_new_settings_from_current_layers() {
        let base = tempfile::tempdir().unwrap();
        let overrides = tempfile::tempdir().unwrap();
        let original = include_str!("../../../etc/parameters/base/ball_filter.json5");
        std::fs::write(base.path().join("ball_filter.json5"), original).unwrap();
        let mut historical: serde_json::Value = json5::from_str(original).unwrap();
        historical
            .as_object_mut()
            .unwrap()
            .remove("visible_missed_detection_timeout");
        historical
            .as_object_mut()
            .unwrap()
            .remove("maximum_obstacle_time_difference");
        historical["maximum_matching_cost"] = serde_json::json!(2.5);
        let legacy: BallFilterParameters = serde_json::from_value(historical.clone()).unwrap();
        assert!(legacy.visible_missed_detection_timeout.is_zero());
        let path = overrides.path().join("ball_filter.json5");
        std::fs::write(&path, serde_json::to_vec(&historical).unwrap()).unwrap();
        let retained = capture_parameter_override(&path).unwrap();
        std::fs::write(&path, serde_json::to_vec(&retained).unwrap()).unwrap();
        let context = ContextBuilder::default()
            .with_namespace("/capture_defaults_test")
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(std::iter::empty::<&str>())
            .with_parameter_layers([base.path().to_owned(), overrides.path().to_owned()])
            .build()
            .await
            .unwrap();
        let node = context.create_node("capture").build().await.unwrap();
        let binding = node
            .bind_parameter_as::<BallFilterParameters>("ball_filter")
            .unwrap();
        let snapshot = binding.snapshot();
        assert_eq!(snapshot.typed().maximum_matching_cost, 2.5);
        assert_eq!(
            snapshot.typed().visible_missed_detection_timeout,
            Duration::from_secs(1)
        );
        assert_eq!(
            snapshot.typed().maximum_obstacle_time_difference,
            Duration::from_millis(100)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn live_best_updates_the_node_atomically_without_changing_baseline() {
        let baseline = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        let original = include_str!("../../../etc/parameters/base/ball_filter.json5");
        std::fs::write(baseline.path().join("ball_filter.json5"), original).unwrap();
        let context = ContextBuilder::default()
            .with_namespace("/live_ball_filter_test")
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(std::iter::empty::<&str>())
            .with_parameter_layers([baseline.path().to_owned(), live.path().to_owned()])
            .build()
            .await
            .unwrap();
        let filter = context.create_node("ball_filter").build().await.unwrap();
        let binding = filter
            .bind_parameter_as::<BallFilterParameters>("ball_filter")
            .unwrap();
        let node = Arc::new(context.create_node("optimizer").build().await.unwrap());
        let client =
            RemoteParameterClient::new(node, "/live_ball_filter_test/ball_filter").unwrap();
        let mut best = binding.snapshot().typed().clone();
        best.maximum_matching_cost = 2.5;
        best.noise.detection_noise.inner.fill(0.125);
        apply_live_parameters(&client, &best).await.unwrap();
        let snapshot = binding.snapshot();
        assert_eq!(snapshot.typed().maximum_matching_cost, 2.5);
        assert_eq!(snapshot.typed().noise.detection_noise.x(), 0.125);
        let revision = client.get_snapshot().await.unwrap().revision;
        best.maximum_matching_cost = 1.75;
        best.noise.detection_noise.inner.fill(0.25);
        apply_live_parameters(&client, &best).await.unwrap();
        assert_eq!(client.get_snapshot().await.unwrap().revision, revision + 1);
        assert_eq!(binding.snapshot().typed().maximum_matching_cost, 1.75);
        assert_eq!(binding.snapshot().typed().noise.detection_noise.x(), 0.25);
        assert_eq!(
            std::fs::read_to_string(baseline.path().join("ball_filter.json5")).unwrap(),
            original
        );
        context.shutdown().unwrap();
    }

    #[test]
    fn field_walls_rebound_fast_balls_on_all_sides_and_at_a_corner() {
        let mut parameters: SimulatorParameters =
            json5::from_str(include_str!("../parameters/simulator.json5")).unwrap();
        parameters.ball.joint_damping = 0.0;
        parameters.ball.joint_friction_loss = 0.0;
        let dimensions = parameters.field_dimensions;
        let bounds = [
            f64::from(dimensions.length / 2.0 + dimensions.border_strip_width),
            f64::from(dimensions.width / 2.0 + dimensions.border_strip_width),
        ];
        for direction in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0], [1.0, 1.0]] {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
            app.insert_resource(SimulationMode::Paused);
            app.world_mut()
                .spawn(crate::scene::walls::object(dimensions));
            let ball = spawn_ball(&mut app, &parameters, [0.0; 3]);
            app.update();
            let mut world = app.world_mut().resource_mut::<MujocoWorld>();
            world.data_mut().model_opt_mut().gravity.fill(0.0);
            world
                .set_object_pose(
                    ball,
                    Transform::from_xyz(
                        (direction[0] * (bounds[0] - 0.7)) as f32,
                        0.5,
                        (-direction[1] * (bounds[1] - 0.7)) as f32,
                    ),
                )
                .unwrap();
            step_with_ball_impulse(
                &mut world,
                Some(BallImpulse {
                    ball,
                    impulse: direction.map(|v| v * 1.8),
                    height_above_center: 0.0,
                }),
            )
            .unwrap();
            for _ in 0..150 {
                step_with_ball_impulse(&mut world, None).unwrap();
                let (position, _) = first_pose(&world, &SpawnedBalls(vec![ball])).unwrap();
                for axis in 0..2 {
                    assert!(
                        position[axis].abs() < bounds[axis],
                        "ball escaped through a wall"
                    );
                }
            }
            let velocity =
                crate::scene::ball::first_velocity(&world, &SpawnedBalls(vec![ball])).unwrap();
            for axis in 0..2 {
                if direction[axis] == 0.0 {
                    continue;
                }
                let rebound = -direction[axis] * velocity[axis];
                assert!(
                    (3.2..4.2).contains(&rebound),
                    "weak or unstable rebound {direction:?}: {velocity:?}"
                );
            }
        }
    }

    #[test]
    fn impulses_add_momentum_and_are_removed_after_one_physics_step() {
        let mut parameters: SimulatorParameters =
            json5::from_str(include_str!("../parameters/simulator.json5")).unwrap();
        // Check momentum conservation without the configured resistive forces.
        parameters.ball.joint_damping = 0.0;
        parameters.ball.joint_friction_loss = 0.0;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(SimulationMode::Paused);
        let ball = spawn_ball(&mut app, &parameters, [0.0; 3]);
        app.update();
        let mut world = app.world_mut().resource_mut::<MujocoWorld>();
        // Isolate impulse integration from contact: use zero gravity and lift the ball.
        world.data_mut().model_opt_mut().gravity.fill(0.0);
        world
            .set_object_pose(ball, Transform::from_xyz(0.0, 1.0, 0.0))
            .unwrap();
        let kick = BallImpulse {
            ball,
            impulse: [1.35, 0.0],
            height_above_center: 0.0,
        };
        let velocity = |world: &MujocoWorld| {
            crate::scene::ball::first_velocity(world, &SpawnedBalls(vec![ball])).unwrap()[0]
        };
        step_with_ball_impulse(&mut world, Some(kick)).unwrap();
        let first = velocity(&world);
        assert!(
            (first - 1.35 / f64::from(parameters.ball.mass)).abs() < 1e-6,
            "velocity after impulse: {first}"
        );
        step_with_ball_impulse(&mut world, None).unwrap();
        assert!(
            (velocity(&world) - first).abs() < 1e-6,
            "impulse must not persist"
        );
        step_with_ball_impulse(&mut world, Some(kick)).unwrap();
        assert!(
            (velocity(&world) - 2.0 * first).abs() < 1e-6,
            "kicks must add momentum, not set speed"
        );
        let reverse = BallImpulse {
            impulse: [-2.7, 0.0],
            ..kick
        };
        step_with_ball_impulse(&mut world, Some(reverse)).unwrap();
        assert!(velocity(&world).abs() < 1e-6);
        let above_center = BallImpulse {
            height_above_center: 0.4 * f64::from(parameters.field_dimensions.ball_radius),
            ..kick
        };
        step_with_ball_impulse(&mut world, Some(above_center)).unwrap();
        let (_, rotation) = first_pose(&world, &SpawnedBalls(vec![ball])).unwrap();
        assert!(
            rotation[2].abs() > 0.001,
            "physical ball must rotate about Y"
        );
        step_with_ball_impulse(&mut world, None).unwrap();
        let (_, next_rotation) = first_pose(&world, &SpawnedBalls(vec![ball])).unwrap();
        assert!(
            next_rotation[2] > rotation[2],
            "spin must continue after the impulse"
        );
    }
}
