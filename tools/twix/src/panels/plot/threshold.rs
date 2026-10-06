use std::{convert::Infallible, sync::Arc, time::Duration};

use color_eyre::eyre::Context as _;
use eframe::egui::{Color32, Context, Ui};
use hulk_widgets::CompletionEdit;
use ros_z::{node::Node, parameter::RemoteParameterClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::task::AbortOnDropHandle;

use crate::{
    backend::RobotBackend,
    panels::parameter::{
        collect_parameter_paths, parameter_node_completions, parameter_node_fqn,
        run_remote_operation,
    },
};

use super::conversion::Conversion;

const RETRY_DELAY: Duration = Duration::from_secs(1);
/// Revisions restart when a parameter node restarts, so events alone cannot
/// tell whether the last snapshot is still current.
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(super) struct SavedThreshold {
    node: String,
    path: String,
    color: Color32,
    visible: bool,
    conversion: Conversion,
}

impl Default for SavedThreshold {
    fn default() -> Self {
        Self {
            node: String::new(),
            path: String::new(),
            color: Color32::GRAY,
            visible: true,
            conversion: Conversion::default(),
        }
    }
}

impl SavedThreshold {
    pub fn with_color(color: Color32) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }
}

/// A numeric parameter drawn as horizontal guide lines, for example a
/// threshold guarding an output.
pub(super) struct Threshold {
    pub id: usize,
    pub source: ParameterSource,
    pub color: Color32,
    pub visible: bool,
    pub conversion: Conversion,
}

impl Threshold {
    pub fn new(id: usize, saved: SavedThreshold) -> Self {
        Self {
            id,
            source: ParameterSource::new(saved.node, saved.path),
            color: saved.color,
            visible: saved.visible,
            conversion: saved.conversion,
        }
    }

    pub fn save(&self) -> SavedThreshold {
        SavedThreshold {
            node: self.source.node.clone(),
            path: self.source.path.clone(),
            color: self.color,
            visible: self.visible,
            conversion: self.conversion.clone(),
        }
    }

    pub fn label(&self) -> String {
        let source = format!("{}: {}", self.source.node, self.source.path);
        match self.conversion.label() {
            Some(conversion) => format!("{source} ({conversion})"),
            None => source,
        }
    }

    /// Converted values of the selected parameter.
    pub fn values(&self) -> Result<Vec<f64>, String> {
        self.source
            .numbers()?
            .into_iter()
            .map(|value| {
                let value = self.conversion.apply_number(value)?;
                if value.is_finite() {
                    Ok(value)
                } else {
                    Err("Converted value is NaN or infinite.".to_owned())
                }
            })
            .collect()
    }

    /// The first unconverted value, which previews the conversion. Arrays
    /// convert each value in the same way.
    pub fn preview_input(&self) -> Option<Result<Value, String>> {
        match self.source.numbers() {
            Ok(values) => Some(Ok(values.first()?.to_owned().into())),
            Err(error) => Some(Err(error)),
        }
    }
}

/// Node and path selection for a parameter. The node's parameter snapshot is
/// followed live, so tuning the parameter moves the line.
pub(super) struct ParameterSource {
    node_editor: String,
    node: String,
    path_editor: String,
    path: String,
    focus_requested: bool,
    watch: Option<ParameterWatch>,
    snapshot: Option<Snapshot>,
    error: Option<String>,
}

struct Snapshot {
    value: Arc<Value>,
    paths: Vec<String>,
}

type Update = Option<Result<Arc<Value>, String>>;

/// Background subscription to one node's parameters. Dropping it aborts the
/// task and releases the event subscription.
struct ParameterWatch {
    target: String,
    updates: watch::Receiver<Update>,
    _task: AbortOnDropHandle<()>,
}

impl ParameterSource {
    fn new(node: String, path: String) -> Self {
        Self {
            node_editor: node.clone(),
            node,
            path_editor: path.clone(),
            path,
            focus_requested: false,
            watch: None,
            snapshot: None,
            error: None,
        }
    }

    pub fn request_focus(&mut self) {
        self.focus_requested = true;
    }

    /// Node and path inputs. Paths complete from the latest snapshot.
    pub fn ui(&mut self, ui: &mut Ui, backend: &RobotBackend) {
        let nodes = {
            let graph = backend.graph().lock();
            parameter_node_completions(
                graph.services().map(|service| &service.topic),
                &backend.namespace(),
                &self.node_editor,
            )
        };
        let paths = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.paths.as_slice())
            .unwrap_or_default();
        let width = ((ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 2.0
            - ui.spacing().interact_size.x)
            .max(40.0);
        ui.spacing_mut().text_edit_width = width;
        ui.label("Node");
        let node = ui
            .add(
                CompletionEdit::new(ui.id().with("node"), &nodes, &mut self.node_editor)
                    .request_focus(std::mem::take(&mut self.focus_requested)),
            )
            .on_hover_text(
                "Parameter node, relative to the robot namespace or absolute. \
                 Ctrl+Space opens completions. Press Enter to apply.",
            );
        // Like topics, apply only on Enter or a chosen completion, so a
        // partial name never starts following a node.
        if node.changed() {
            self.node = self.node_editor.trim().to_owned();
        }
        ui.label("Path");
        // Fill the rest of the row, so the input ends where topic inputs do.
        ui.spacing_mut().text_edit_width = f32::INFINITY;
        let path = ui
            .add(CompletionEdit::new(
                ui.id().with("path"),
                paths,
                &mut self.path_editor,
            ))
            .on_hover_text(
                "Dot-separated path to a number or an array of numbers. Press Enter to apply.",
            );
        if path.changed() {
            self.path = self.path_editor.trim().to_owned();
        }
    }

    /// Follow the selected node in the current namespace, restarting the
    /// subscription when either changes.
    pub fn reconcile(&mut self, backend: &RobotBackend, egui_context: &Context) {
        let target = parameter_node_fqn(&backend.namespace(), &self.node);
        if self.watch.as_ref().map(|watch| &watch.target) == target.as_ref() {
            return;
        }
        self.snapshot = None;
        self.error = None;
        self.watch = target.map(|target| ParameterWatch::spawn(backend, egui_context, target));
    }

    /// Apply the latest snapshot. Errors keep the last known value displayed.
    pub fn refresh(&mut self) {
        let Some(watch) = &mut self.watch else {
            return;
        };
        if !watch.updates.has_changed().unwrap_or(false) {
            return;
        }
        match watch.updates.borrow_and_update().clone() {
            Some(Ok(value)) => {
                self.snapshot = Some(Snapshot {
                    paths: collect_parameter_paths(&value),
                    value,
                });
                self.error = None;
            }
            Some(Err(error)) => self.error = Some(error),
            None => {}
        }
    }

    fn numbers(&self) -> Result<Vec<f64>, String> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| "Waiting for parameters.".to_owned())?;
        numbers_at_path(&snapshot.value, &self.path)
    }

    pub fn status(&self) -> String {
        match (&self.watch, &self.snapshot) {
            (None, _) => "Enter a parameter node and path.".to_owned(),
            (Some(watch), None) => format!("Waiting for parameters of {}", watch.target),
            (Some(watch), Some(_)) => format!("Following parameters of {}", watch.target),
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl ParameterWatch {
    fn spawn(backend: &RobotBackend, egui_context: &Context, target: String) -> Self {
        let (sender, updates) = watch::channel(None);
        let task = backend.runtime_handle().spawn(watch_parameters(
            backend.node(),
            target.clone(),
            sender,
            egui_context.clone(),
        ));
        Self {
            target,
            updates,
            _task: AbortOnDropHandle::new(task),
        }
    }
}

async fn watch_parameters(
    node: Arc<Node>,
    target: String,
    sender: watch::Sender<Update>,
    egui_context: Context,
) {
    loop {
        let Err(error) = follow_snapshots(&node, &target, &sender, &egui_context).await;
        sender.send_replace(Some(Err(error)));
        egui_context.request_repaint();
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Fetch a snapshot, then fetch again whenever an event announces a newer
/// revision or the refresh interval elapses. Subscribing first ensures no
/// change is missed in between.
async fn follow_snapshots(
    node: &Arc<Node>,
    target: &str,
    sender: &watch::Sender<Update>,
    egui_context: &Context,
) -> Result<Infallible, String> {
    let client = RemoteParameterClient::new(node.clone(), target)
        .map_err(|error| format!("failed to create remote parameter client: {error}"))?;
    let events = run_remote_operation("subscribing to parameter events", async {
        client
            .subscribe_events()
            .await
            .wrap_err("failed to subscribe to parameter events")
    })
    .await?;
    loop {
        let response = run_remote_operation("fetching parameter snapshot", async {
            client
                .get_snapshot()
                .await
                .wrap_err("failed to fetch parameter snapshot")
        })
        .await?;
        if !response.success {
            return Err(response.message);
        }
        let value: Value = serde_json::from_str(&response.value_json)
            .map_err(|error| format!("failed to parse parameter snapshot: {error}"))?;
        let changed = sender.send_if_modified(|update| match update {
            Some(Ok(current)) if **current == value => false,
            _ => {
                *update = Some(Ok(Arc::new(value)));
                true
            }
        });
        if changed {
            egui_context.request_repaint();
        }
        let refresh = tokio::time::Instant::now() + REFRESH_INTERVAL;
        while let Ok(event) = tokio::time::timeout_at(refresh, events.recv()).await {
            let event =
                event.map_err(|error| format!("parameter event subscription closed: {error}"))?;
            if event.revision > response.revision {
                break;
            }
        }
    }
}

/// Look up a dot-separated parameter path, matching the node's own path
/// semantics, and accept a number or an array of numbers.
fn numbers_at_path(root: &Value, path: &str) -> Result<Vec<f64>, String> {
    if path.is_empty() {
        return Err("Enter a parameter path.".to_owned());
    }
    let value = path
        .split('.')
        .try_fold(root, |value, segment| value.get(segment))
        .ok_or_else(|| format!("Parameter not found: {path}"))?;
    let not_numeric = || format!("Parameter {path} is not a number or an array of numbers.");
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| value.as_f64().ok_or_else(not_numeric))
            .collect(),
        value => Ok(vec![value.as_f64().ok_or_else(not_numeric)?]),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ros_z::{
        context::ContextBuilder,
        parameter::{NodeParameters, NodeParametersExt},
    };
    use serde_json::json;

    use super::*;

    #[derive(Clone, Serialize, Deserialize, ros_z::Message)]
    #[message(name = "twix_test::ThresholdParameters")]
    struct ThresholdParameters {
        ball: BallParameters,
    }

    #[derive(Clone, Serialize, Deserialize, ros_z::Message)]
    #[message(name = "twix_test::BallParameters")]
    struct BallParameters {
        confidence_threshold: f64,
    }

    async fn next_value(updates: &mut watch::Receiver<Update>) -> Value {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                updates.changed().await.expect("watch task is running");
                if let Some(Ok(value)) = &*updates.borrow_and_update() {
                    return value.as_ref().clone();
                }
            }
        })
        .await
        .expect("parameter update within timeout")
    }

    /// Parameter node in its own context, so shutting the context down
    /// behaves like a process exit.
    async fn start_parameter_node(
        endpoint: &str,
        layers: &[PathBuf],
    ) -> (
        ros_z::context::Context,
        Node,
        NodeParameters<ThresholdParameters>,
    ) {
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints([endpoint])
            .with_parameter_layers(layers.iter().cloned())
            .build()
            .await
            .unwrap();
        let node = context
            .create_node("ball_detection")
            .with_namespace("robot")
            .build()
            .await
            .unwrap();
        let parameters = node
            .bind_parameter_as::<ThresholdParameters>("ball_detection")
            .unwrap();
        (context, node, parameters)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn watch_follows_parameter_changes_and_restarts() {
        let root = std::env::temp_dir().join(format!("twix_threshold_{}", std::process::id()));
        let base = root.join("base");
        let robot = root.join("robot");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("ball_detection.json5"),
            "{ ball: { confidence_threshold: 0.5 } }",
        )
        .unwrap();
        let layers = [base, robot.clone()];
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let endpoint = format!("tcp/127.0.0.1:{port}");
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_listen_endpoints([endpoint.as_str()])
            .build()
            .await
            .unwrap();
        let client = Arc::new(context.create_node("twix").build().await.unwrap());
        let server = start_parameter_node(&endpoint, &layers).await;

        let (sender, mut updates) = watch::channel(None);
        let _task = AbortOnDropHandle::new(tokio::spawn(watch_parameters(
            client.clone(),
            "/robot/ball_detection".to_owned(),
            sender,
            Context::default(),
        )));
        let value = next_value(&mut updates).await;
        assert_eq!(
            numbers_at_path(&value, "ball.confidence_threshold"),
            Ok(vec![0.5])
        );

        let set = RemoteParameterClient::new(client, "/robot/ball_detection")
            .unwrap()
            .set_json(
                "ball.confidence_threshold",
                &json!(0.8),
                robot.to_string_lossy(),
                None,
            )
            .await
            .unwrap();
        assert!(set.success, "{}", set.message);
        let value = next_value(&mut updates).await;
        assert_eq!(
            numbers_at_path(&value, "ball.confidence_threshold"),
            Ok(vec![0.8])
        );

        // A restarted node starts again at revision zero without new events.
        server.0.shutdown().unwrap();
        drop(server);
        std::fs::write(
            robot.join("ball_detection.json5"),
            "{ ball: { confidence_threshold: 0.3 } }",
        )
        .unwrap();
        let _server = start_parameter_node(&endpoint, &layers).await;
        let value = next_value(&mut updates).await;
        assert_eq!(
            numbers_at_path(&value, "ball.confidence_threshold"),
            Ok(vec![0.3])
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn numbers_are_read_from_nested_paths() {
        let root = json!({ "ball": { "confidence_threshold": 0.7, "range": [1, 2.5] } });

        assert_eq!(
            numbers_at_path(&root, "ball.confidence_threshold"),
            Ok(vec![0.7])
        );
        assert_eq!(numbers_at_path(&root, "ball.range"), Ok(vec![1.0, 2.5]));
    }

    #[test]
    fn missing_and_non_numeric_parameters_are_errors() {
        let root = json!({ "ball": { "enabled": true, "range": [1, "far"] } });

        assert_eq!(
            numbers_at_path(&root, "ball.radius"),
            Err("Parameter not found: ball.radius".to_owned())
        );
        assert!(numbers_at_path(&root, "ball.enabled").is_err());
        assert!(numbers_at_path(&root, "ball.range").is_err());
        assert!(numbers_at_path(&root, "").is_err());
    }
}
