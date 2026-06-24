use std::{
    convert::Into, env::current_dir, path::PathBuf, str::FromStr, sync::Arc, time::SystemTime,
};

use argument_parsers::RobotAddress;
use clap::Parser;
use color_eyre::{
    Report, Result,
    eyre::{Context as _, ContextCompat, eyre},
};
use eframe::{
    App, CreationContext, Frame, NativeOptions, Renderer, Storage,
    egui::{
        CentralPanel, Context, CornerRadius, Id, Label, Layout, Sense, StrokeKind, TopBottomPanel,
        Ui, Widget, WidgetText,
    },
    egui_wgpu::{WgpuConfiguration, WgpuSetup},
    emath::Align,
    epaint::Color32,
    run_native,
};
use egui_dock::{
    DockArea, DockState, LeafNode, Node, NodeIndex, Split, SurfaceIndex, TabAddAlign, TabIndex,
};
use serde_json::{Value, from_str, to_string};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use configuration::{
    Configuration,
    keybind_plugin::{self, KeybindSystem},
    keys::KeybindAction,
};
use log::{error, warn};
use panel::{Panel, PanelCreationContext};
use panels::{EnumPlotPanel, MapPanel, PlotPanel, TextPanel, UnsupportedPanel};
use repository::{Repository, inspect_version::check_for_update};
use visuals::Visuals;

use crate::backend::{TwixBackend, connection::ConnectionStatus};

mod backend;
mod configuration;
mod panel;
mod panels;
mod topic_completion_edit;
mod twix_painter;
mod visuals;
mod zoom_and_pan;

const DEFAULT_ROUTER_ENDPOINT: &str = "tcp/127.0.0.1:7447";
const DEFAULT_TARGET_NAMESPACE: &str = "/42";

#[derive(Debug, Parser)]
struct Arguments {
    /// robot number or address whose ros-z namespace should be inspected
    pub address: Option<String>,
    /// Alternative repository root
    #[arg(long)]
    repository_root: Option<PathBuf>,
    /// Delete the current panel setup
    #[arg(long)]
    pub clear: bool,
}

fn setup_logger() -> Result<(), Report> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,bevy_render=warn"));

    let layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_level(true)
        .with_file(false)
        .with_line_number(true)
        .compact();

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
        .wrap_err("failed to initialize tracing subscriber")?;

    Ok(())
}

fn main() -> Result<(), eframe::Error> {
    setup_logger().expect("failed to setup logger");

    let arguments = Arguments::parse();
    let repository = arguments
        .repository_root
        .clone()
        .map(Repository::new)
        .map(Ok)
        .unwrap_or_else(|| {
            let current_directory = current_dir().wrap_err("failed to get current directory")?;
            Repository::find_root(current_directory).wrap_err("failed to find repository root")
        });
    match &repository {
        Ok(repository) => {
            if let Err(error) = check_for_update(
                env!("CARGO_PKG_VERSION"),
                repository.root.join("tools/twix/Cargo.toml"),
                "twix",
            ) {
                error!("{error:#?}");
            }
        }
        Err(error) => {
            warn!("{error:#?}");
        }
    }

    let configuration = Configuration::load().unwrap_or_else(|error| {
        warn!("failed to load configuration, falling back to defaults: {error}");
        Configuration::default()
    });

    let mut wgpu_options = WgpuConfiguration::default();
    match &mut wgpu_options.wgpu_setup {
        WgpuSetup::CreateNew(wgpu_setup_create_new) => {
            let old_closure = wgpu_setup_create_new.device_descriptor.clone();
            wgpu_setup_create_new.device_descriptor = Arc::new(move |adapter| {
                let mut old = (old_closure)(adapter);
                old.required_limits.max_storage_buffers_per_shader_stage = 9;
                old
            })
        }
        WgpuSetup::Existing(_wgpu_setup_existing) => unimplemented!(),
    }
    run_native(
        "Twix",
        NativeOptions {
            wgpu_options,
            renderer: Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(|creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            Ok(Box::new(TwixApp::create(
                creation_context,
                arguments,
                configuration,
                repository.ok(),
            )?))
        }),
    )
}

fn storage_string(storage: Option<&dyn Storage>, key: &str) -> Option<String> {
    storage.and_then(|storage| storage.get_string(key))
}

fn robot_address_to_namespace(address: &str) -> Result<String> {
    let address_without_port = address
        .split_once(':')
        .map_or(address, |(address_without_port, _port)| {
            address_without_port
        });
    let robot_address = RobotAddress::from_str(address_without_port)
        .wrap_err_with(|| format!("failed to parse robot number/address '{address}'"))?;
    let [first, second, third, robot_number] = robot_address.ip.octets();
    if first == 10 && matches!(second, 0 | 1) && third != 0 {
        Ok(format!("/{robot_number}"))
    } else {
        Err(eyre!(
            "robot number/address '{address}' does not resolve to a robot-network address"
        ))
    }
}

fn robot_address_to_router_endpoint(address: &str) -> Result<String> {
    let (address_without_port, port) = address
        .split_once(':')
        .map_or((address, "7447"), |(address_without_port, port)| {
            (address_without_port, port)
        });
    let robot_address = RobotAddress::from_str(address_without_port)
        .wrap_err_with(|| format!("failed to parse robot number/address '{address}'"))?;

    robot_address_to_namespace(address)?;

    Ok(format!("tcp/{}:{port}", robot_address.ip))
}

fn router_endpoint_from_storage(storage: Option<&dyn Storage>) -> String {
    let Some(router_endpoint) = storage_string(storage, "router_endpoint") else {
        if let Some(address) = storage_string(storage, "address") {
            return match robot_address_to_router_endpoint(&address) {
                Ok(router_endpoint) => router_endpoint,
                Err(error) => {
                    warn!(
                        "invalid saved robot address '{address}', falling back to \
                         {DEFAULT_ROUTER_ENDPOINT}: {error:#}"
                    );
                    DEFAULT_ROUTER_ENDPOINT.to_string()
                }
            };
        }
        return DEFAULT_ROUTER_ENDPOINT.to_string();
    };

    match TwixBackend::validate_router_endpoint(&router_endpoint) {
        Ok(()) => router_endpoint,
        Err(error) => {
            warn!(
                "invalid saved router endpoint '{router_endpoint}', falling back to \
                 {DEFAULT_ROUTER_ENDPOINT}: {error:#}"
            );
            DEFAULT_ROUTER_ENDPOINT.to_string()
        }
    }
}

fn router_endpoint_from_arguments_and_storage(
    argument_address: Option<&str>,
    storage: Option<&dyn Storage>,
) -> String {
    if let Some(address) = argument_address {
        return match robot_address_to_router_endpoint(address) {
            Ok(router_endpoint) => router_endpoint,
            Err(error) => {
                warn!(
                    "invalid command line robot number/address '{address}', falling back to \
                     {DEFAULT_ROUTER_ENDPOINT}: {error:#}"
                );
                DEFAULT_ROUTER_ENDPOINT.to_string()
            }
        };
    }

    router_endpoint_from_storage(storage)
}

fn keep_connected_from_storage_value(value: Option<String>) -> bool {
    value.as_deref() != Some("false")
}

fn keep_connected_from_storage(storage: Option<&dyn Storage>) -> bool {
    keep_connected_from_storage_value(storage_string(storage, "keep_connected"))
}

fn normalize_target_namespace_or_default(namespace: &str, source: &str) -> String {
    match backend::topic::normalize_namespace(namespace) {
        Ok(namespace) => namespace,
        Err(error) => {
            warn!(
                "invalid {source} target namespace '{namespace}', falling back to \
                 {DEFAULT_TARGET_NAMESPACE}: {error:#}"
            );
            DEFAULT_TARGET_NAMESPACE.to_string()
        }
    }
}

fn target_namespace_from_arguments_and_storage(
    argument_address: Option<&str>,
    storage: Option<&dyn Storage>,
) -> String {
    if let Some(address) = argument_address {
        return match robot_address_to_namespace(address) {
            Ok(namespace) => normalize_target_namespace_or_default(&namespace, "command line"),
            Err(error) => {
                warn!(
                    "invalid command line robot number/address '{address}', falling back to \
                     {DEFAULT_TARGET_NAMESPACE}: {error:#}"
                );
                DEFAULT_TARGET_NAMESPACE.to_string()
            }
        };
    }

    if let Some(namespace) = storage_string(storage, "target_namespace") {
        return normalize_target_namespace_or_default(&namespace, "saved");
    }

    if let Some(address) = storage_string(storage, "address") {
        return match robot_address_to_namespace(&address) {
            Ok(namespace) => normalize_target_namespace_or_default(&namespace, "migrated saved"),
            Err(error) => {
                warn!(
                    "invalid saved robot address '{address}', falling back to \
                     {DEFAULT_TARGET_NAMESPACE}: {error:#}"
                );
                DEFAULT_TARGET_NAMESPACE.to_string()
            }
        };
    }

    DEFAULT_TARGET_NAMESPACE.to_string()
}

fn create_backend_or_default_with<Backend>(
    router_endpoint: &mut String,
    target_namespace: &mut String,
    mut create_backend: impl FnMut(&str, &str) -> Result<Backend>,
) -> Result<Backend> {
    match create_backend(router_endpoint, target_namespace) {
        Ok(backend) => Ok(backend),
        Err(error) => {
            warn!(
                "failed to create Twix ros-z backend for router '{router_endpoint}' and namespace \
                 '{target_namespace}', falling back to defaults: {error:#}"
            );
            *router_endpoint = DEFAULT_ROUTER_ENDPOINT.to_string();
            *target_namespace = DEFAULT_TARGET_NAMESPACE.to_string();
            create_backend(DEFAULT_ROUTER_ENDPOINT, DEFAULT_TARGET_NAMESPACE)
                .wrap_err("failed to create Twix ros-z backend with default settings")
        }
    }
}

fn create_backend_or_default(
    router_endpoint: &mut String,
    target_namespace: &mut String,
    keep_connected: bool,
    egui_context: Context,
) -> Result<Arc<TwixBackend>> {
    create_backend_or_default_with(
        router_endpoint,
        target_namespace,
        |router_endpoint, target_namespace| {
            let backend = if keep_connected {
                TwixBackend::new(router_endpoint, target_namespace, egui_context.clone())
            } else {
                TwixBackend::new_with_keep_connected(
                    router_endpoint,
                    target_namespace,
                    egui_context.clone(),
                    false,
                )
            };
            backend.map(Arc::new)
        },
    )
}

enum SelectablePanel {
    Text(TextPanel),
    Plot(PlotPanel),
    EnumPlot(EnumPlotPanel),
    Map(Box<MapPanel>),
    Unsupported(UnsupportedPanel),
}

impl SelectablePanel {
    fn new(context: PanelCreationContext) -> Result<SelectablePanel> {
        let value = context.value.ok_or(eyre!("Got none value"))?;
        Self::from_saved_value(value, || context)
    }

    fn from_saved_value<'a>(
        saved_value: &'a Value,
        context: impl FnOnce() -> PanelCreationContext<'a>,
    ) -> Result<SelectablePanel> {
        if let Some(panel) = Self::legacy_unsupported_from_saved_value(saved_value) {
            return Ok(panel);
        }

        let name = saved_value
            .get("_panel_type")
            .ok_or(eyre!("value has no _panel_type: {saved_value:?}"))?
            .as_str()
            .ok_or(eyre!("_panel_type is not a string"))?
            .to_owned();
        Self::from_saved_name(&name, context())
    }

    fn legacy_unsupported_from_saved_value(saved_value: &Value) -> Option<SelectablePanel> {
        let panel_type = saved_value.get("type")?.as_str()?;
        Some(SelectablePanel::Unsupported(UnsupportedPanel::new(
            panel_type,
            Some(saved_value),
        )))
    }

    pub fn try_from_name<'a>(
        panel_name: &str,
        context: impl FnOnce() -> PanelCreationContext<'a>,
    ) -> Result<SelectablePanel> {
        match panel_name {
            TextPanel::NAME => Ok(SelectablePanel::Text(TextPanel::new(context()))),
            PlotPanel::NAME => Ok(SelectablePanel::Plot(PlotPanel::new(context()))),
            EnumPlotPanel::NAME => Ok(SelectablePanel::EnumPlot(EnumPlotPanel::new(context()))),
            MapPanel::NAME => Ok(SelectablePanel::Map(Box::new(MapPanel::new(context())))),
            other => Err(eyre!("unknown panel '{other}'")),
        }
    }

    fn from_saved_name(panel_name: &str, context: PanelCreationContext) -> Result<SelectablePanel> {
        match panel_name {
            TextPanel::NAME => Ok(SelectablePanel::Text(TextPanel::new(context))),
            PlotPanel::NAME => Ok(SelectablePanel::Plot(PlotPanel::new(context))),
            EnumPlotPanel::NAME => Ok(SelectablePanel::EnumPlot(EnumPlotPanel::new(context))),
            MapPanel::NAME => Ok(SelectablePanel::Map(Box::new(MapPanel::new(context)))),
            other => Ok(SelectablePanel::Unsupported(UnsupportedPanel::new(
                other,
                context.value,
            ))),
        }
    }

    pub fn registered() -> Vec<String> {
        vec![
            TextPanel::NAME.to_owned(),
            PlotPanel::NAME.to_owned(),
            EnumPlotPanel::NAME.to_owned(),
            MapPanel::NAME.to_owned(),
        ]
    }

    pub fn save(&self) -> Value {
        let mut value = match self {
            SelectablePanel::Text(panel) => panel.save(),
            SelectablePanel::Plot(panel) => panel.save(),
            SelectablePanel::EnumPlot(panel) => panel.save(),
            SelectablePanel::Map(panel) => panel.save(),
            SelectablePanel::Unsupported(panel) => return panel.save(),
        };
        value["_panel_type"] = Value::String(self.to_string());
        value
    }
}

impl Widget for &mut SelectablePanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        match self {
            SelectablePanel::Text(panel) => panel.ui(ui),
            SelectablePanel::Plot(panel) => panel.ui(ui),
            SelectablePanel::EnumPlot(panel) => panel.ui(ui),
            SelectablePanel::Map(panel) => panel.ui(ui),
            SelectablePanel::Unsupported(panel) => panel.ui(ui),
        }
    }
}

impl std::fmt::Display for SelectablePanel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let panel_name = match self {
            SelectablePanel::Text(_) => TextPanel::NAME,
            SelectablePanel::Plot(_) => PlotPanel::NAME,
            SelectablePanel::EnumPlot(_) => EnumPlotPanel::NAME,
            SelectablePanel::Map(_) => MapPanel::NAME,
            SelectablePanel::Unsupported(panel) => panel.title(),
        };
        formatter.write_str(panel_name)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;

    #[derive(Default)]
    struct MemoryStorage {
        values: HashMap<String, String>,
    }

    impl Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.values.insert(key.to_string(), value);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn robot_argument_selects_robot_router_endpoint() {
        assert_eq!(
            router_endpoint_from_arguments_and_storage(Some("42"), None),
            "tcp/10.1.24.42:7447"
        );
        assert_eq!(
            router_endpoint_from_arguments_and_storage(Some("42w"), None),
            "tcp/10.0.24.42:7447"
        );
    }

    #[test]
    fn robot_address_on_non_hulks_team_selects_robot_namespace() {
        assert_eq!(robot_address_to_namespace("10.1.7.21").unwrap(), "/21");
        assert_eq!(robot_address_to_namespace("10.0.12.8").unwrap(), "/8");
    }

    #[test]
    fn saved_legacy_address_migrates_router_endpoint_when_missing() {
        let mut storage = MemoryStorage::default();
        storage.set_string("address", "42w".to_string());

        assert_eq!(
            router_endpoint_from_arguments_and_storage(None, Some(&storage)),
            "tcp/10.0.24.42:7447"
        );
    }

    #[test]
    fn saved_router_endpoint_takes_precedence_over_legacy_address() {
        let mut storage = MemoryStorage::default();
        storage.set_string("address", "42w".to_string());
        storage.set_string("router_endpoint", "tcp/127.0.0.1:7448".to_string());

        assert_eq!(
            router_endpoint_from_arguments_and_storage(None, Some(&storage)),
            "tcp/127.0.0.1:7448"
        );
    }

    #[test]
    fn backend_creation_returns_default_error_when_default_backend_fails() {
        let mut router_endpoint = "tcp/10.1.24.42:7447".to_string();
        let mut target_namespace = "/42".to_string();
        let mut calls = Vec::new();

        let result = create_backend_or_default_with(
            &mut router_endpoint,
            &mut target_namespace,
            |router_endpoint, target_namespace| {
                calls.push((router_endpoint.to_string(), target_namespace.to_string()));
                Err::<(), _>(color_eyre::eyre::eyre!(
                    "backend failed for {router_endpoint} {target_namespace}"
                ))
            },
        );

        assert!(result.is_err());
        assert_eq!(router_endpoint, DEFAULT_ROUTER_ENDPOINT);
        assert_eq!(target_namespace, DEFAULT_TARGET_NAMESPACE);
        assert_eq!(
            calls,
            [
                ("tcp/10.1.24.42:7447".to_string(), "/42".to_string()),
                (
                    DEFAULT_ROUTER_ENDPOINT.to_string(),
                    DEFAULT_TARGET_NAMESPACE.to_string()
                ),
            ]
        );
        assert!(
            format!("{:#}", result.unwrap_err())
                .contains("failed to create Twix ros-z backend with default settings")
        );
    }

    #[test]
    fn live_panel_constructor_rejects_unregistered_text() {
        let Err(error) = SelectablePanel::try_from_name("Behavior Simulator", || {
            unreachable!("unknown panels must be rejected before a backend is needed")
        }) else {
            panic!("unknown panel was accepted");
        };

        assert!(format!("{error:#}").contains("unknown panel"));
    }

    #[test]
    fn registered_panels_include_map() {
        assert!(SelectablePanel::registered().contains(&"Map".to_string()));
    }

    #[test]
    fn saved_legacy_panel_loads_as_unsupported_and_saves_original_json() {
        let saved_panel = json!({
            "type": "Image",
            "value": {
                "path": "Vision.main_outputs.ycbcr422_image",
                "overlay": "Ball Detection"
            }
        });

        let panel = SelectablePanel::from_saved_value(&saved_panel, || {
            unreachable!("legacy unsupported panels must not need a live backend")
        })
        .expect("legacy panel should load as unsupported");

        assert!(matches!(panel, SelectablePanel::Unsupported(_)));
        assert_eq!(panel.save(), saved_panel);
    }

    #[test]
    fn keep_connected_defaults_to_true_without_saved_value() {
        assert!(keep_connected_from_storage_value(None));
    }

    #[test]
    fn keep_connected_reads_saved_false() {
        assert!(!keep_connected_from_storage_value(Some(
            "false".to_string()
        )));
    }

    #[test]
    fn save_does_not_apply_router_endpoint_to_backend() {
        let backend = Arc::new(
            TwixBackend::new_with_keep_connected(
                "tcp/127.0.0.1:7447",
                "/42",
                Context::default(),
                false,
            )
            .unwrap(),
        );
        let mut app = TwixApp {
            backend: backend.clone(),
            router_endpoint: "tcp/127.0.0.1:7448".to_string(),
            last_valid_router_endpoint: "tcp/127.0.0.1:7447".to_string(),
            keep_connected: false,
            target_namespace: "/42".to_string(),
            panel_selection: TextPanel::NAME.to_string(),
            last_focused_tab: (0.into(), 0.into()),
            dock_state: DockState::new(vec![
                SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                    backend: backend.clone(),
                    value: None,
                }))
                .into(),
            ]),
            visual: Visuals::Dark,
        };
        let mut storage = MemoryStorage::default();

        app.save(&mut storage);

        assert_eq!(backend.router_endpoint(), "tcp/127.0.0.1:7447");
        assert_eq!(
            storage.get_string("router_endpoint").as_deref(),
            Some("tcp/127.0.0.1:7448")
        );
        assert_eq!(
            storage.get_string("keep_connected").as_deref(),
            Some("false")
        );
    }
}

struct TwixApp {
    backend: Arc<TwixBackend>,
    router_endpoint: String,
    last_valid_router_endpoint: String,
    keep_connected: bool,
    target_namespace: String,
    panel_selection: String,
    last_focused_tab: (NodeIndex, TabIndex),
    dock_state: DockState<Tab>,
    visual: Visuals,
}

impl TwixApp {
    fn create(
        creation_context: &CreationContext,
        arguments: Arguments,
        configuration: Configuration,
        _repository: Option<Repository>,
    ) -> Result<Self> {
        let mut router_endpoint = router_endpoint_from_arguments_and_storage(
            arguments.address.as_deref(),
            creation_context.storage,
        );
        let mut target_namespace = target_namespace_from_arguments_and_storage(
            arguments.address.as_deref(),
            creation_context.storage,
        );
        let keep_connected = keep_connected_from_storage(creation_context.storage);
        let backend = create_backend_or_default(
            &mut router_endpoint,
            &mut target_namespace,
            keep_connected,
            creation_context.egui_ctx.clone(),
        )?;
        target_namespace = backend.target_namespace();

        let dock_state: Option<DockState<Value>> = if arguments.clear {
            None
        } else {
            creation_context
                .storage
                .and_then(|storage| storage.get_string("dock_state"))
                .and_then(|string| from_str(&string).ok())
        };

        let dock_state = match dock_state {
            Some(dock_state) => dock_state.map_tabs(|value| {
                Tab::new(PanelCreationContext {
                    backend: backend.clone(),
                    value: Some(value),
                })
            }),
            None => DockState::new(vec![
                SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                    backend: backend.clone(),
                    value: None,
                }))
                .into(),
            ]),
        };

        let context = creation_context.egui_ctx.clone();

        keybind_plugin::register(&context);
        context.set_keybinds(Arc::new(configuration.keys));

        let visual = creation_context
            .storage
            .and_then(|storage| storage.get_string("style"))
            .and_then(|theme| Visuals::from_str(&theme).ok())
            .unwrap_or(Visuals::Dark);
        visual.set_visual(&creation_context.egui_ctx);

        let panel_selection = "".to_string();

        Ok(Self {
            backend,
            last_valid_router_endpoint: router_endpoint.clone(),
            router_endpoint,
            keep_connected,
            target_namespace,
            panel_selection,
            dock_state,
            last_focused_tab: (0.into(), 0.into()),
            visual,
        })
    }

    fn focus_left(&mut self, node_id: NodeIndex, surface_index: SurfaceIndex) -> Option<()> {
        let parent_id = node_id.parent()?;
        let parent = &self.dock_state[surface_index][parent_id];
        if node_id.is_left() || parent.is_vertical() {
            return self.focus_left(parent_id, surface_index);
        }
        let mut left_id = parent_id.left();

        loop {
            let node = &self.dock_state[surface_index][left_id];
            match node {
                Node::Empty => unreachable!("cannot hit an empty node while digging down"),
                Node::Leaf { .. } => break,
                Node::Vertical { .. } => {
                    left_id = left_id.left();
                }
                Node::Horizontal { .. } => {
                    left_id = left_id.right();
                }
            };
        }

        self.dock_state
            .set_focused_node_and_surface((surface_index, left_id));
        Some(())
    }

    fn focus_right(&mut self, node_id: NodeIndex, surface_index: SurfaceIndex) -> Option<()> {
        let parent_id = node_id.parent()?;
        let parent = &self.dock_state[surface_index][parent_id];
        if node_id.is_right() || parent.is_vertical() {
            return self.focus_right(parent_id, surface_index);
        }
        let mut child = parent_id.right();

        loop {
            let node = &self.dock_state[surface_index][child];
            match node {
                Node::Empty => unreachable!("cannot hit an empty node while digging down"),
                Node::Leaf { .. } => break,
                Node::Vertical { .. } => {
                    child = child.left();
                }
                Node::Horizontal { .. } => {
                    child = child.left();
                }
            };
        }

        self.dock_state
            .set_focused_node_and_surface((surface_index, child));
        Some(())
    }

    fn focus_above(&mut self, node_id: NodeIndex, surface_index: SurfaceIndex) -> Option<()> {
        let parent_id = node_id.parent()?;
        let parent = &self.dock_state[surface_index][parent_id];
        if node_id.is_left() || parent.is_horizontal() {
            return self.focus_above(parent_id, surface_index);
        }
        let mut left_id = parent_id.left();

        loop {
            let node = &self.dock_state[surface_index][left_id];
            match node {
                Node::Empty => unreachable!("cannot hit an empty node while digging down"),
                Node::Leaf { .. } => break,
                Node::Vertical { .. } => {
                    left_id = left_id.right();
                }
                Node::Horizontal { .. } => {
                    left_id = left_id.left();
                }
            };
        }

        self.dock_state
            .set_focused_node_and_surface((surface_index, left_id));
        Some(())
    }

    fn focus_below(&mut self, node_id: NodeIndex, surface_index: SurfaceIndex) -> Option<()> {
        let parent_id = node_id.parent()?;
        let parent = &self.dock_state[surface_index][parent_id];
        if node_id.is_right() || parent.is_horizontal() {
            return self.focus_below(parent_id, surface_index);
        }
        let mut child = parent_id.right();

        loop {
            let node = &self.dock_state[surface_index][child];
            match node {
                Node::Empty => unreachable!("cannot hit an empty node while digging down"),
                Node::Leaf { .. } => break,
                Node::Vertical { .. } => {
                    child = child.left();
                }
                Node::Horizontal { .. } => {
                    child = child.left();
                }
            };
        }

        self.dock_state
            .set_focused_node_and_surface((surface_index, child));
        Some(())
    }
}

impl App for TwixApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("Router");
                    let router_response = ui.text_edit_singleline(&mut self.router_endpoint);
                    if router_response.changed()
                        && TwixBackend::validate_router_endpoint(&self.router_endpoint).is_ok()
                    {
                        self.last_valid_router_endpoint = self.router_endpoint.clone();
                    }
                    if router_response.lost_focus() {
                        match self
                            .backend
                            .set_router_endpoint(self.router_endpoint.clone())
                        {
                            Ok(()) => {
                                self.router_endpoint = self.backend.router_endpoint();
                                self.last_valid_router_endpoint = self.router_endpoint.clone();
                            }
                            Err(error) => {
                                error!("invalid router endpoint: {error:#}");
                                self.router_endpoint = self.last_valid_router_endpoint.clone();
                                ui.label(
                                    "invalid router endpoint; keeping previous valid endpoint",
                                );
                            }
                        };
                    }

                    if ui
                        .checkbox(&mut self.keep_connected, "Keep connected")
                        .changed()
                    {
                        self.backend.set_keep_connected(self.keep_connected);
                    }
                    let connection_status = match self.backend.connection_status() {
                        ConnectionStatus::Disconnected => "disconnected",
                        ConnectionStatus::Connecting => "connecting",
                        ConnectionStatus::Connected => "connected",
                        ConnectionStatus::Failed => "connection failed",
                    };
                    let connection_response = ui.label(connection_status);
                    if let Some(message) = self.backend.connection_unavailable_message() {
                        connection_response.on_hover_text(message);
                    }

                    ui.label("Namespace");
                    let namespace_response = ui.text_edit_singleline(&mut self.target_namespace);
                    if namespace_response.changed() {
                        match self.backend.set_target_namespace(&self.target_namespace) {
                            Ok(()) => {
                                self.target_namespace = self.backend.target_namespace();
                            }
                            Err(error) => {
                                error!("invalid target namespace: {error:#}");
                                self.target_namespace = self.backend.target_namespace();
                            }
                        }
                    }

                    if self.active_tab_index() != Some(self.last_focused_tab) {
                        self.last_focused_tab =
                            self.active_tab_index().unwrap_or((0.into(), 0.into()));
                        if let Some(name) = self
                            .active_tab()
                            .and_then(|tab| tab.panel.as_ref().ok())
                            .map(|panel| format!("{panel}"))
                        {
                            self.panel_selection = name
                        }
                    }
                    let panels = SelectablePanel::registered();
                    let panel_input = ui.text_edit_singleline(&mut self.panel_selection);
                    let mut panel_selection_changed = panel_input.changed();
                    ui.menu_button("Panels", |ui| {
                        for panel in panels {
                            if ui.button(&panel).clicked() {
                                self.panel_selection = panel;
                                panel_selection_changed = true;
                            }
                        }
                    });

                    if context.keybind_pressed(KeybindAction::FocusPanel) {
                        panel_input.request_focus();
                    }
                    if panel_selection_changed {
                        match SelectablePanel::try_from_name(&self.panel_selection, || {
                            PanelCreationContext {
                                backend: self.backend.clone(),
                                value: None,
                            }
                        }) {
                            Ok(panel) => {
                                if let Some(active_tab) = self.active_tab() {
                                    active_tab.panel = Ok(panel);
                                }
                            }
                            Err(err) => error!("{err:?}"),
                        }
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button("⚙", |ui| {
                        ui.menu_button("Theme", |ui| {
                            ui.vertical(|ui| {
                                for visual in Visuals::iter() {
                                    if ui.button(visual.to_string()).clicked() {
                                        self.visual = visual;
                                        self.visual.set_visual(context);
                                    }
                                }
                            })
                        });
                    });
                });
            })
        });
        CentralPanel::default().show(context, |ui| {
            if context.keybind_pressed(KeybindAction::OpenSplit) {
                let tab = SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                    backend: self.backend.clone(),
                    value: None,
                }));
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    let node = &mut self.dock_state[surface_index][node_id];
                    if node.tabs_count() == 0 {
                        node.append_tab(tab.into());
                    } else {
                        let rect = node.rect().unwrap();
                        let direction = if rect.height() > rect.width() {
                            Split::Below
                        } else {
                            Split::Right
                        };
                        self.dock_state.split(
                            (surface_index, node_id),
                            direction,
                            0.5,
                            Node::leaf(tab.into()),
                        );
                    }
                }
            }
            if context.keybind_pressed(KeybindAction::OpenTab) {
                let tab = SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                    backend: self.backend.clone(),
                    value: None,
                }));
                self.dock_state.push_to_focused_leaf(tab.into());
            }

            if context.keybind_pressed(KeybindAction::FocusLeft)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                self.focus_left(node_id, surface_index);
            }
            if context.keybind_pressed(KeybindAction::FocusBelow)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                self.focus_below(node_id, surface_index);
            }
            if context.keybind_pressed(KeybindAction::FocusAbove)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                self.focus_above(node_id, surface_index);
            }
            if context.keybind_pressed(KeybindAction::FocusRight)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                self.focus_right(node_id, surface_index);
            }

            if context.keybind_pressed(KeybindAction::DuplicateTab)
                && let Some((_, tab)) = self.dock_state.find_active_focused()
            {
                let new_tab = tab.save();
                self.dock_state.push_to_focused_leaf(Tab::from(
                    SelectablePanel::new(PanelCreationContext {
                        backend: self.backend.clone(),
                        value: Some(&new_tab),
                    })
                    .unwrap(),
                ));
            }

            if context.keybind_pressed(KeybindAction::CloseTab)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                let active_node = &mut self.dock_state[surface_index][node_id];
                if let Node::Leaf(LeafNode { active, tabs, .. }) = active_node
                    && !tabs.is_empty()
                {
                    tabs.remove(active.0);

                    active.0 = active.0.saturating_sub(1);

                    if tabs.is_empty() && node_id != NodeIndex(0) {
                        self.dock_state[surface_index].remove_leaf(node_id);
                    }
                }
            }

            if context.keybind_pressed(KeybindAction::CloseAll) {
                self.dock_state = DockState::new(vec![
                    SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                        backend: self.backend.clone(),
                        value: None,
                    }))
                    .into(),
                ]);
                self.last_focused_tab = (0.into(), 0.into());
                self.dock_state
                    .set_focused_node_and_surface((0.into(), 0.into()));
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.buttons.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer::default();
            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut tab_viewer);

            for (surface_index, node_id) in tab_viewer.nodes_to_add_tabs_to {
                let tab = SelectablePanel::Text(TextPanel::new(PanelCreationContext {
                    backend: self.backend.clone(),
                    value: None,
                }));
                let index = self.dock_state[surface_index][node_id].tabs_count();
                self.dock_state[surface_index][node_id].insert_tab(index.into(), tab.into());
                self.dock_state
                    .set_focused_node_and_surface((surface_index, node_id));
            }

            if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                let node = &self.dock_state[surface_index][node_id];
                if let Some(rect) = node.rect() {
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(4),
                        ui.visuals().widgets.active.bg_stroke,
                        StrokeKind::Outside,
                    );
                }
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let dock_state = self.dock_state.map_tabs(|tab| tab.save());
        if TwixBackend::validate_router_endpoint(&self.router_endpoint).is_ok() {
            self.last_valid_router_endpoint = self.router_endpoint.clone();
        }
        if let Err(error) = TwixBackend::validate_router_endpoint(&self.last_valid_router_endpoint)
        {
            warn!(
                "invalid last valid router endpoint '{}', falling back to {DEFAULT_ROUTER_ENDPOINT}: {error:#}",
                self.last_valid_router_endpoint
            );
            self.last_valid_router_endpoint = DEFAULT_ROUTER_ENDPOINT.to_string();
        }
        self.target_namespace = self.backend.target_namespace();

        storage.set_string("dock_state", to_string(&dock_state).unwrap());
        storage.set_string(
            "router_endpoint",
            self.last_valid_router_endpoint.to_string(),
        );
        storage.set_string("keep_connected", self.keep_connected.to_string());
        storage.set_string("target_namespace", self.target_namespace.to_string());
        storage.set_string("style", self.visual.to_string());
    }
}

impl TwixApp {
    fn active_tab(&mut self) -> Option<&mut Tab> {
        let (_viewport, tab) = self.dock_state.find_active_focused()?;
        Some(tab)
    }

    fn active_tab_index(&self) -> Option<(NodeIndex, TabIndex)> {
        let (surface, node) = self.dock_state.focused_leaf()?;
        if let Node::Leaf(LeafNode { active, .. }) = &self.dock_state[surface][node] {
            Some((node, *active))
        } else {
            None
        }
    }
}

struct Tab {
    id: Id,
    panel: Result<SelectablePanel, (Report, Option<Value>)>,
}

impl From<SelectablePanel> for Tab {
    fn from(panel: SelectablePanel) -> Self {
        Self {
            id: Id::new(SystemTime::now()),
            panel: Ok(panel),
        }
    }
}

impl Tab {
    fn new(context: PanelCreationContext) -> Self {
        let value = context.value.cloned();
        Self {
            id: Id::new(SystemTime::now()),
            panel: SelectablePanel::new(context).map_err(|error| (error, value)),
        }
    }

    fn save(&self) -> Value {
        match &self.panel {
            Ok(panel) => panel.save(),
            Err((_report, value)) => value.clone().unwrap_or_default(),
        }
    }
}

#[derive(Default)]
struct TabViewer {
    nodes_to_add_tabs_to: Vec<(SurfaceIndex, NodeIndex)>,
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match &mut tab.panel {
            Ok(panel) => panel.ui(ui),

            Err((error, value)) => {
                ui.label(format!("Error loading panel: {error}"));
                ui.collapsing("JSON", |ui| {
                    let content = match serde_json::to_string_pretty(value) {
                        Ok(pretty_string) => pretty_string,
                        Err(error) => error.to_string(),
                    };
                    let label = ui.add(Label::new(&content).sense(Sense::click()));
                    if label.clicked() {
                        ui.ctx().copy_text(content);
                    }
                    label.on_hover_ui_at_pointer(|ui| {
                        ui.label("Click to copy");
                    });
                })
                .header_response
            }
        };
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        match &mut tab.panel {
            Ok(panel) => format!("{panel}").into(),
            Err((error, _value)) => WidgetText::from(format!("{error}")).color(Color32::LIGHT_RED),
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        tab.id
    }

    fn on_add(&mut self, surface_index: SurfaceIndex, node: NodeIndex) {
        self.nodes_to_add_tabs_to.push((surface_index, node));
    }
}
