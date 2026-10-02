use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use eframe::egui::{self, Ui};
use linear_algebra::{Isometry2, Point3, Pose2, point, vector};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, TopicObservation};
use tokio::{sync::oneshot, task::JoinHandle};
use twix_visualization::twix_painter::{Orientation, TwixPainter};
use types::ball_filter_tuning::{
    Metrics, NAMESPACE, OPEN_VIEWER_TOPIC, PROGRESS_TOPIC, Progress, ROUTER,
    TUNED_PARAMETER_POINTERS,
};
use types::{
    ball_position::BallPosition, field_dimensions::FieldDimensions, time_wrapper::TimeWrapper,
};

use crate::{
    backend::RobotBackend,
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationRepaint, RepaintOnUpdates},
};

pub struct BallFilterOptimizationPanel {
    connection: Option<Connection>,
    pending: Option<PendingConnection>,
    error: Option<String>,
    viewer_request: Option<oneshot::Receiver<Result<()>>>,
}

struct PendingConnection {
    receiver: oneshot::Receiver<Result<Arc<RobotBackend>>>,
    task: JoinHandle<()>,
}

impl Drop for PendingConnection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Connection {
    progress: TopicObservation<Progress>,
    truth: TopicObservation<TimeWrapper<Vec<Point3<Field>>>>,
    obstacles: TopicObservation<TimeWrapper<Vec<nalgebra::Point3<f32>>>>,
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    field_dimensions: TopicObservation<FieldDimensions>,
    estimate: TopicObservation<Option<BallPosition<Ground>>>,
    _repaint: ObservationRepaint,
    _backend: Arc<RobotBackend>,
    sample: Option<Arc<SampleRecord<Progress>>>,
    received_at: Instant,
    history: Vec<(u64, f64)>,
    run: String,
}

impl Panel for BallFilterOptimizationPanel {
    const STORAGE_ID: &'static str = "ball_filter_optimization";
    const DISPLAY_NAME: &'static str = "Ball-filter optimization";
    const ICON: &'static str = egui_material_icons::icons::ICON_TUNE.codepoint;

    fn new(_context: PanelCreationContext<'_>) -> Self {
        Self {
            connection: None,
            pending: None,
            error: None,
            viewer_request: None,
        }
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        ui.strong("Ball-filter optimization");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.pending.is_none(),
                    egui::Button::new("Connect to simulator / optimizer"),
                )
                .clicked()
            {
                self.connection = None;
                self.error = None;
                let runtime = context.backend.runtime_handle().clone();
                let handle = runtime.clone();
                let repaint = context.egui_context.clone();
                let (sender, receiver) = oneshot::channel();
                let task = runtime.spawn(async move {
                    let result = RobotBackend::new(handle, Some(ROUTER.into()), NAMESPACE.into())
                        .await
                        .map(Arc::new);
                    let _ = sender.send(result);
                    repaint.request_repaint();
                });
                self.pending = Some(PendingConnection { receiver, task });
            }
            if let Some(connection) = &self.connection {
                if ui
                    .add_enabled(
                        self.viewer_request.is_none(),
                        egui::Button::new("Open 3D view"),
                    )
                    .clicked()
                {
                    let node = connection._backend.node();
                    let repaint = context.egui_context.clone();
                    let (sender, receiver) = oneshot::channel();
                    context.backend.runtime_handle().spawn(async move {
                        let result = async {
                            let publisher =
                                node.publisher::<bool>(OPEN_VIEWER_TOPIC).build().await?;
                            publisher.publish(&true).await?;
                            Ok::<_, color_eyre::Report>(())
                        };
                        let result = tokio::time::timeout(Duration::from_secs(3), result)
                            .await
                            .map_err(color_eyre::Report::from)
                            .and_then(|result| result);
                        let _ = sender.send(result);
                        repaint.request_repaint();
                    });
                    self.viewer_request = Some(receiver);
                }
            }
            if self.connection.is_some() && ui.button("Disconnect").clicked() {
                self.connection = None;
            }
        });
        if let Some(pending) = &mut self.pending {
            match pending.receiver.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result.and_then(|backend| {
                        let progress = observe_progress(&backend)?;
                        let repaint = progress.repaint_on_updates(&PanelUiContext {
                            backend: &backend,
                            egui_context: context.egui_context,
                        });
                        let truth = backend
                            .observer()
                            .observe_typed("simulation/ball_ground_truth_field")?
                            .spawn();
                        let estimate = backend
                            .observer()
                            .observe_typed("ball_filter/ball_position")?
                            .spawn();
                        let obstacles = backend
                            .observer()
                            .observe_typed("simulation/obstacle_positions_world")?
                            .spawn();
                        let ground_to_field = backend
                            .observer()
                            .observe_typed("ground_to_field")?
                            .policy(ros_z_debug::ObservationPolicy::time_window(
                                Duration::from_secs(1),
                            )?)
                            .spawn();
                        let field_dimensions = backend
                            .observer()
                            .observe_typed("field_dimensions")?
                            .policy(
                                ros_z_debug::ObservationPolicy::latest().with_subscriber_qos(
                                    ros_z::qos::QosProfile {
                                        durability: ros_z::qos::QosDurability::TransientLocal,
                                        ..Default::default()
                                    },
                                ),
                            )
                            .spawn();
                        Ok(Connection {
                            truth,
                            obstacles,
                            ground_to_field,
                            field_dimensions,
                            estimate,
                            progress,
                            _repaint: repaint,
                            _backend: backend,
                            sample: None,
                            received_at: Instant::now(),
                            history: Vec::new(),
                            run: String::new(),
                        })
                    }) {
                        Ok(connection) => self.connection = Some(connection),
                        Err(error) => self.error = Some(format!("{error:#}")),
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    ui.spinner();
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.pending = None;
                }
            }
        }
        if let Some(request) = &mut self.viewer_request {
            match request.try_recv() {
                Ok(result) => {
                    self.viewer_request = None;
                    if let Err(error) = result {
                        self.error = Some(format!("Could not request 3D view: {error:#}"));
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.viewer_request = None;
                }
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let Some(connection) = &mut self.connection else {
            ui.label("Connect to an optimizer started with:");
            monospace(
                ui,
                "./simulator --tune-ball-filter logs/my-run --keep-tuning-open",
            );
            return;
        };
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        let latest = connection.progress.latest();
        if let Some(sample) = latest {
            if connection
                .sample
                .as_ref()
                .is_none_or(|old| !Arc::ptr_eq(old, &sample))
            {
                connection.received_at = Instant::now();
                if connection.run != sample.value.output_directory {
                    connection.run = sample.value.output_directory.clone();
                    connection.history.clear();
                }
                if let Some(search) = &sample.value.search {
                    if connection
                        .history
                        .last()
                        .is_none_or(|(trial, _)| *trial != search.trial)
                    {
                        connection.history.push((search.trial, search.best.loss));
                    }
                }
                connection.sample = Some(sample);
            }
        }
        let Some(sample) = &connection.sample else {
            ui.label("Waiting for the local simulator / optimizer…");
            monospace(ui, ROUTER);
            match connection.progress.status() {
                ros_z_debug::TopicObservationStatus::Retrying { error, .. } => {
                    ui.label(format!("Retrying connection: {error}"));
                }
                ros_z_debug::TopicObservationStatus::Blocked { reason, .. } => {
                    ui.label(format!("Connection blocked: {reason:?}"));
                }
                ros_z_debug::TopicObservationStatus::Observing { cache } => {
                    ui.label("Observing optimizer progress");
                    if let Some(message) = cache.message() {
                        ui.label(message);
                    }
                }
                _ => {
                    ui.label("Connecting…");
                }
            }
            return;
        };
        let age = connection.received_at.elapsed().as_secs_f32();
        if age > 3.0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("No updates for {age:.0}s — showing the last received state."),
            );
        } else {
            ui.label("Connected to local simulator / optimizer");
        }
        let progress = &sample.value;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.strong(&progress.status);
            if let Some(status) = &progress.viewer_status {
                ui.label(status);
            }
            if let Some(status) = &progress.live_status {
                ui.label(status);
                ui.label(&progress.phase);
            }
            if let Some(error) = &progress.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            if progress.search.is_none() && progress.live_status.is_none() {
                ui.label(format!(
                    "Recording {} / {}: {}",
                    progress.recording_index, progress.recordings, progress.recording
                ));
                ui.label(&progress.phase);
                ui.add(
                    egui::ProgressBar::new(
                        (progress.elapsed_seconds / progress.duration_seconds.max(1.0)) as f32,
                    )
                    .text(format!(
                        "{:.1} / {:.0} seconds",
                        progress.elapsed_seconds, progress.duration_seconds
                    )),
                );
            }
            if progress.status == "Recording" || progress.live_trial.is_some() {
                live_ball(ui, connection);
            }
            if let Some(search) = &progress.search {
                if search.trials == 0 {
                    ui.label(format!("Trial {} · continuous search", search.trial));
                } else {
                    ui.add(
                        egui::ProgressBar::new(search.trial as f32 / search.trials.max(1) as f32)
                            .text(format!("Trial {} / {}", search.trial, search.trials)),
                    );
                }
                ui.label(format!(
                    "Training recordings · {} coordinates",
                    search.reference_frame
                ));
                scores(ui, "training", &search.baseline, &search.best);
                loss_plot(
                    ui,
                    &connection.history,
                    search.trials.max(search.trial),
                    search.baseline.loss,
                );
                if let (Some(baseline), Some(best)) =
                    (&search.validation_baseline, &search.validation_best)
                {
                    ui.separator();
                    ui.label("Held-out recordings (not used to choose parameters)");
                    scores(ui, "validation", baseline, best);
                }
                ui.label(
                    "Loss includes position error, missed balls and false tracks. Lower is better.",
                );
                if let Ok(mut fixed) = serde_json::to_value(&search.best_parameters) {
                    let mut tuned = serde_json::Map::new();
                    for pointer in TUNED_PARAMETER_POINTERS {
                        let (parent, key) =
                            pointer.rsplit_once('/').expect("static parameter pointer");
                        if let Some(value) = fixed
                            .pointer_mut(parent)
                            .and_then(serde_json::Value::as_object_mut)
                            .and_then(|map| map.remove(key))
                        {
                            tuned.insert(pointer.trim_start_matches('/').replace('/', "."), value);
                        }
                    }
                    ui.collapsing("Best tuned values", |ui| {
                        ui.label("10 search variables; x/y noise values are coupled.");
                        if let Ok(json) = serde_json::to_string_pretty(&tuned) {
                            monospace(ui, json);
                        }
                    });
                    ui.collapsing("Fixed values (not searched)", |ui| {
                        ui.label("maximum_matching_cost_validity_penalty_factor is retained for old configuration compatibility and is no longer used.");
                        if let Ok(json) = serde_json::to_string_pretty(&fixed) {
                            monospace(ui, json);
                        }
                    });
                }
            }
            ui.separator();
            ui.label("Recordings and results:");
            monospace(ui, &progress.output_directory);
        });
    }
}

fn monospace(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    ui.label(egui::RichText::new(text).monospace().size(size))
}

fn observe_progress(backend: &RobotBackend) -> Result<TopicObservation<Progress>> {
    use ros_z::qos::{QosDurability, QosHistory, QosProfile};
    Ok(backend
        .observer()
        .observe_typed(PROGRESS_TOPIC)?
        .policy(
            ros_z_debug::ObservationPolicy::latest().with_subscriber_qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            }),
        )
        .spawn())
}

fn scores(ui: &mut Ui, id: &str, baseline: &Metrics, best: &Metrics) {
    egui::Grid::new(id).striped(true).show(ui, |ui| {
        ui.label("");
        ui.strong("Baseline");
        ui.strong("Best");
        ui.end_row();
        for (name, a, b) in [
            ("Loss", Some(baseline.loss), Some(best.loss)),
            (
                "Position RMSE (m)",
                baseline.position_rmse_metres,
                best.position_rmse_metres,
            ),
            (
                "Close-range RMSE (m)",
                baseline.close_range_position_rmse_metres,
                best.close_range_position_rmse_metres,
            ),
            (
                "Close-range missing (s)",
                Some(baseline.close_range_missing_seconds),
                Some(best.close_range_missing_seconds),
            ),
            (
                "Along-motion lag (ms)",
                baseline.motion_lag_seconds.map(|value| value * 1000.0),
                best.motion_lag_seconds.map(|value| value * 1000.0),
            ),
            (
                "Motion-lag coverage (s)",
                Some(baseline.moving_reference_seconds),
                Some(best.moving_reference_seconds),
            ),
            (
                "Missing ball (s)",
                Some(baseline.missing_seconds),
                Some(best.missing_seconds),
            ),
            (
                "Missing ground-to-field (s)",
                Some(baseline.missing_transform_seconds),
                Some(best.missing_transform_seconds),
            ),
            (
                "Longest missing gap (s)",
                Some(baseline.longest_missing_seconds),
                Some(best.longest_missing_seconds),
            ),
            (
                "False track (s)",
                Some(baseline.false_track_seconds),
                Some(best.false_track_seconds),
            ),
        ] {
            let label = ui.label(name);
            if name == "Loss" {
                label.on_hover_text("Single-ball position and availability loss. Reference balls outside the field receive smoothly decreasing weight with distance beyond the boundary (decay length 0.3 m). Raw error and missing-time metrics below are not downweighted.");
            } else if name == "Missing ball (s)" {
                label.on_hover_text("Total labelled time when a real ball exists but no filter estimate is available in the scoring frame. Includes balls outside the camera view and, for field scoring, estimates without a matching ground-to-field transform. Lower is better.");
            } else if name == "Longest missing gap (s)" {
                label.on_hover_text("Longest continuous labelled interval with a real ball but no usable estimate. Includes startup and missing field transforms; this is not specifically the time to recover after a kick.");
            } else if name == "Along-motion lag (ms)" {
                label.on_hover_text("Signed along-motion position error divided by reference speed at the same timestamp. Positive means behind the ball. Uses adjacent single-ball references in field coordinates with valid motion; multi-ball and missing estimates are excluded. This measures spatial lag, not message delivery latency.");
            } else if name == "Close-range RMSE (m)" {
                label.on_hover_text("Position error against the nearest labelled ball within 1 m of the robot, conditional on an available estimate. Read together with close-range missing time.");
            }
            ui.label(a.map_or_else(|| "—".into(), |v| format!("{v:.4}")));
            ui.label(b.map_or_else(|| "—".into(), |v| format!("{v:.4}")));
            ui.end_row();
        }
    });
}

fn loss_plot(ui: &mut Ui, history: &[(u64, f64)], trials: u64, baseline: f64) {
    if history.len() < 2 {
        return;
    }
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::hover(),
    );
    let rect = response.rect.shrink(6.0);
    let points = history
        .iter()
        .map(|(trial, loss)| {
            egui::pos2(
                rect.left() + rect.width() * *trial as f32 / trials.max(1) as f32,
                rect.bottom()
                    - rect.height() * (loss / baseline.max(f64::EPSILON)).clamp(0.0, 1.0) as f32,
            )
        })
        .collect();
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
    );
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
    ));
    ui.label("Best training loss over trials (received since connecting)");
}

fn recent_pose(connection: &Connection, time: Time) -> Option<Isometry2<Ground, Field>> {
    connection
        .ground_to_field
        .get_nearest(time)
        .filter(|pose| pose.source_time.as_nanos().abs_diff(time.as_nanos()) <= 20_000_000)
        .map(|pose| pose.value)
}

fn live_ball(ui: &mut Ui, connection: &Connection) {
    let Some(dimensions) = connection.field_dimensions.latest() else {
        ui.label("Waiting for field dimensions…");
        return;
    };
    let Some(truth) = connection.truth.latest() else {
        return;
    };
    let robot_pose = recent_pose(connection, truth.value.time);
    let estimate = connection.estimate.latest().filter(|estimate| {
        estimate
            .source_time
            .as_nanos()
            .abs_diff(truth.value.time.as_nanos())
            <= 100_000_000
    });
    let estimate_in_field = estimate.as_ref().and_then(|estimate| {
        let ball = estimate.value.as_ref()?;
        let pose = recent_pose(connection, estimate.source_time)?;
        Some(pose * ball.position)
    });
    let dimensions = &dimensions.value;
    let border = dimensions.border_strip_width;
    let length = dimensions.length + 2.0 * border;
    let width = dimensions.width + 2.0 * border;
    let pixel_width = ui.available_width().min(360.0 * length / width);
    let margin = (ui.available_width() - pixel_width) / 2.0;
    ui.horizontal(|ui| {
        ui.add_space(margin);
        ui.allocate_ui(
            egui::vec2(pixel_width, pixel_width * width / length),
            |ui| {
                let (_, painter) = TwixPainter::<Field>::allocate(
                    ui,
                    vector![length, width],
                    point![length / 2.0, -width / 2.0],
                    Orientation::RightHanded,
                );
                painter.field(dimensions);
                if let Some(pose) = robot_pose {
                    painter.pose(
                        Pose2::new(pose * point![0.0, 0.0], pose.inner.rotation.angle()),
                        0.15,
                        0.4,
                        egui::Color32::WHITE,
                        egui::Stroke::new(0.035, egui::Color32::BLACK),
                    );
                }
                for ball in &truth.value.inner {
                    painter.circle_stroke(
                        ball.xy(),
                        0.14,
                        egui::Stroke::new(0.045, egui::Color32::GREEN),
                    );
                }
                if let Some(obstacles) = connection.obstacles.latest().filter(|sample| {
                    sample
                        .value
                        .time
                        .as_nanos()
                        .abs_diff(truth.value.time.as_nanos())
                        <= 100_000_000
                }) {
                    for obstacle in &obstacles.value.inner {
                        painter.circle_filled(
                            point![obstacle.x, obstacle.y],
                            0.22,
                            egui::Color32::from_rgb(255, 115, 13),
                        );
                    }
                }
                if let Some(ball) = estimate_in_field {
                    painter.circle_filled(ball, 0.08, egui::Color32::LIGHT_BLUE);
                }
            },
        );
    });
    ui.label("Field coordinates · white: robot · green: true balls · blue: live filter · orange: opponents");
    if let Some(estimate) = connection.estimate.latest() {
        let age_ms = (truth.value.time.as_nanos() - estimate.source_time.as_nanos()) as f64 / 1e6;
        ui.label(format!("Filter state is {age_ms:.0} ms behind the latest physics sample"))
            .on_hover_text("Timestamp difference, separate from tracking error. The model is displayed at its recorded state time without extrapolation; fusion and transport can contribute to this age.");
    }
    if robot_pose.is_none() {
        ui.label("Robot pose unavailable: no ground-to-field transform within 20 ms.");
    }
    ui.label(
        "Search scores use single-ball recordings; this map shows the unscored live simulation.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercise the actual network path used by the button, including joining a
    // publisher after its first update and decoding the complete typed message.
    #[tokio::test(flavor = "multi_thread")]
    async fn local_optimizer_progress_reaches_twix_when_connecting_late() {
        use ros_z::{
            prelude::*,
            qos::{QosDurability, QosHistory},
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let server = ContextBuilder::default()
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints([endpoint.as_str()])
            .build()
            .await
            .unwrap();
        let node = server
            .create_node("test_optimizer")
            .with_namespace(NAMESPACE)
            .build()
            .await
            .unwrap();
        let publisher = node
            .publisher::<Progress>(PROGRESS_TOPIC)
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await
            .unwrap();
        publisher
            .publish(&Progress {
                status: "Complete".into(),
                search: Some(types::ball_filter_tuning::SearchProgress {
                    trial: 32,
                    trials: 32,
                    best: Metrics {
                        loss: 0.5,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await
            .unwrap();
        let backend = RobotBackend::new(
            tokio::runtime::Handle::current(),
            Some(endpoint),
            NAMESPACE.into(),
        )
        .await
        .unwrap();
        let observation = observe_progress(&backend).unwrap();
        let received = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(sample) = observation.latest() {
                    break sample;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("late Twix observer should receive retained progress");
        assert_eq!(received.value.status, "Complete");
        let search = received.value.search.as_ref().unwrap();
        assert_eq!(search.trial, 32);
        assert_eq!(search.best.loss, 0.5);
        drop(observation);
        drop(backend);
        drop(publisher);
        drop(node);
        server.shutdown().unwrap();
    }
}
