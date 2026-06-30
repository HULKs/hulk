use std::{env::current_dir, path::PathBuf, str::FromStr, sync::Arc};

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context as _, ContextCompat as _},
};
use configuration::{
    Configuration,
    keybind_plugin::{self, KeybindSystem},
    keys::KeybindAction,
};
use eframe::{
    App, CreationContext, Frame, NativeOptions, Storage,
    egui::{CentralPanel, Context, CornerRadius, Layout, StrokeKind, TopBottomPanel, Ui},
    emath::Align,
    run_native,
};
use egui_dock::{
    DockArea, DockState, LeafNode, Node, NodeIndex, Split, SurfaceIndex, TabAddAlign, TabIndex,
};
use hulk_widgets::CompletionEdit;
use log::{error, warn};
use panel::{Panel, PanelCreationContext, PanelUiContext};
use panels::{ImagePanel, TextPanel};
use repository::{Repository, inspect_version::check_for_update};
use serde_json::{Value, from_str, to_string};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use visuals::Visuals;

use crate::backend::RobotBackend;

mod backend;
mod configuration;
mod panel;
mod panels;
mod repaint;
mod selectable_panel_macro;
mod status;
mod visuals;

impl_selectable_panel!(TextPanel, ImagePanel);

fn panel_creation_context<'a>(
    backend: &Arc<RobotBackend>,
    value: Option<&'a Value>,
    egui_context: &Context,
) -> PanelCreationContext<'a> {
    PanelCreationContext {
        backend: backend.clone(),
        value,
        egui_context: egui_context.clone(),
    }
}

fn default_dock_state(backend: &Arc<RobotBackend>, egui_context: &Context) -> DockState<Tab> {
    DockState::new(vec![Tab::from_panel(SelectablePanel::TextPanel(
        TextPanel::new(panel_creation_context(backend, None, egui_context)),
    ))])
}

#[derive(Debug, Clone, clap::Parser)]
struct Arguments {
    /// Target ROS-Z namespace, for example /42.
    namespace: Option<String>,

    /// Router endpoint passed to ROS-Z, for example tcp/127.0.0.1:7447.
    #[arg(long)]
    router: Option<String>,

    /// Alternative repository root for local Twix version checks.
    #[arg(long)]
    repository_root: Option<PathBuf>,

    /// Delete the current panel setup.
    #[arg(long)]
    clear: bool,
}

struct TwixApp {
    dock_state: DockState<Tab>,
    namespace_editor: String,
    panel_selection: String,
    last_focused_tab: (NodeIndex, TabIndex),
    visual: Visuals,
    backend: Arc<RobotBackend>,
    runtime: tokio::runtime::Runtime,
}

impl TwixApp {
    fn create(
        creation_context: &CreationContext,
        arguments: Arguments,
        runtime: tokio::runtime::Runtime,
        backend: Arc<RobotBackend>,
        configuration: Configuration,
    ) -> Self {
        let namespace_editor = backend.namespace();

        let egui_context = creation_context.egui_ctx.clone();
        let dock_state = if arguments.clear {
            None
        } else {
            creation_context
                .storage
                .and_then(|storage| storage.get_string("dock_state"))
                .and_then(|string| match from_str::<DockState<Value>>(&string) {
                    Ok(dock_state) => {
                        let mut load_error = None;
                        let dock_state = dock_state.map_tabs(|value| {
                            let tab = Tab::new(panel_creation_context(
                                &backend,
                                Some(value),
                                &egui_context,
                            ));
                            if let Err(error) = &tab.panel {
                                load_error.get_or_insert_with(|| format!("{error:#}"));
                            }
                            tab
                        });

                        if let Some(error) = load_error {
                            error!("failed to load dock tabs: {error}");
                            None
                        } else {
                            Some(dock_state)
                        }
                    }
                    Err(error) => {
                        error!("failed to load dock state: {error:#}");
                        None
                    }
                })
        }
        .unwrap_or_else(|| default_dock_state(&backend, &egui_context));

        keybind_plugin::register(&creation_context.egui_ctx);
        creation_context
            .egui_ctx
            .set_keybinds(Arc::new(configuration.keys));

        let visual = creation_context
            .storage
            .and_then(|storage| storage.get_string("style"))
            .and_then(|theme| Visuals::from_str(&theme).ok())
            .unwrap_or(Visuals::Dark);
        visual.set_visual(&creation_context.egui_ctx);

        Self {
            dock_state,
            namespace_editor,
            panel_selection: SelectablePanel::registered()
                .first()
                .copied()
                .unwrap_or_default()
                .to_string(),
            last_focused_tab: (0.into(), 0.into()),
            visual,
            backend,
            runtime,
        }
    }

    fn panel_context<'a>(
        &self,
        value: Option<&'a Value>,
        egui_context: &Context,
    ) -> PanelCreationContext<'a> {
        panel_creation_context(&self.backend, value, egui_context)
    }

    fn new_text_tab(&self, egui_context: &Context) -> Tab {
        Tab::from_panel(SelectablePanel::TextPanel(TextPanel::new(
            self.panel_context(None, egui_context),
        )))
    }

    fn default_dock_state(&self, egui_context: &Context) -> DockState<Tab> {
        DockState::new(vec![self.new_text_tab(egui_context)])
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

impl App for TwixApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        let _runtime_guard = self.runtime.enter();

        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("Namespace:");
                    let namespace_response = ui.text_edit_singleline(&mut self.namespace_editor);
                    if context.keybind_pressed(KeybindAction::FocusNamespace) {
                        namespace_response.request_focus();
                    }
                    if namespace_response.lost_focus()
                        && self.namespace_editor != self.backend.namespace()
                        && let Err(error) =
                            self.backend.set_namespace(self.namespace_editor.clone())
                    {
                        log::error!("failed to set namespace: {error:#}");
                        self.namespace_editor = self.backend.namespace();
                    }

                    if self.active_tab_index() != Some(self.last_focused_tab) {
                        self.last_focused_tab =
                            self.active_tab_index().unwrap_or((0.into(), 0.into()));
                        if let Some(name) = self
                            .active_tab()
                            .and_then(|tab| tab.panel.as_ref().ok())
                            .map(|panel| format!("{panel}"))
                        {
                            self.panel_selection = name;
                        }
                    }

                    let panels = SelectablePanel::registered();
                    let panel_input = ui.add(CompletionEdit::new(
                        ui.id().with("panel-selector"),
                        &panels,
                        &mut self.panel_selection,
                    ));

                    if context.keybind_pressed(KeybindAction::FocusPanel) {
                        panel_input.request_focus();
                    }
                    if panel_input.changed() {
                        match SelectablePanel::try_from_display_name(
                            &self.panel_selection,
                            self.panel_context(None, ui.ctx()),
                        ) {
                            Ok(panel) => {
                                if let Some(active_tab) = self.active_tab() {
                                    active_tab.panel = Ok(panel);
                                }
                            }
                            Err(error) => error!("{error:#}"),
                        }
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button("Settings", |ui| {
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
            if context.keybind_pressed(KeybindAction::OpenSplit)
                && let Some((surface_index, node_id)) = self.dock_state.focused_leaf()
            {
                let tab = self.new_text_tab(ui.ctx());
                let node = &mut self.dock_state[surface_index][node_id];
                if node.tabs_count() == 0 {
                    node.append_tab(tab);
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
                        Node::leaf(tab),
                    );
                }
            }
            if context.keybind_pressed(KeybindAction::OpenTab) {
                let tab = self.new_text_tab(ui.ctx());
                self.dock_state.push_to_focused_leaf(tab);
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

            let saved_tab = if context.keybind_pressed(KeybindAction::DuplicateTab) {
                self.dock_state
                    .find_active_focused()
                    .map(|(_, tab)| tab.save())
            } else {
                None
            };
            if let Some(saved_tab) = saved_tab {
                match SelectablePanel::new(self.panel_context(Some(&saved_tab), ui.ctx())) {
                    Ok(panel) => self.dock_state.push_to_focused_leaf(Tab::from_panel(panel)),
                    Err(error) => error!("failed to duplicate tab: {error:#}"),
                }
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
                self.dock_state = self.default_dock_state(ui.ctx());
                self.last_focused_tab = (0.into(), 0.into());
                self.dock_state
                    .set_focused_node_and_surface((0.into(), 0.into()));
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.buttons.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer {
                nodes_to_add_tabs_to: Vec::new(),
                backend: self.backend.clone(),
                egui_context: ui.ctx().clone(),
            };
            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut tab_viewer);

            for (surface_index, node_id) in tab_viewer.nodes_to_add_tabs_to {
                let tab = self.new_text_tab(ui.ctx());
                let index = self.dock_state[surface_index][node_id].tabs_count();
                self.dock_state[surface_index][node_id].insert_tab(index.into(), tab);
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

        storage.set_string(
            "dock_state",
            to_string(&dock_state).expect("dock state should serialize"),
        );
        storage.set_string("namespace", self.backend.namespace());
        storage.set_string("style", self.visual.to_string());
    }
}

struct Tab {
    panel: color_eyre::Result<SelectablePanel>,
    ui_id: uuid::Uuid,
}

impl Tab {
    fn new(context: PanelCreationContext<'_>) -> Self {
        Self {
            panel: SelectablePanel::new(context),
            ui_id: uuid::Uuid::new_v4(),
        }
    }

    fn from_panel(panel: SelectablePanel) -> Self {
        Self {
            panel: Ok(panel),
            ui_id: uuid::Uuid::new_v4(),
        }
    }

    fn save(&self) -> serde_json::Value {
        match &self.panel {
            Ok(panel) => panel.save(),
            Err(error) => serde_json::json!({
                "kind": "load-error",
                "state": { "error": format!("{error:#}") },
            }),
        }
    }
}

struct TabViewer {
    nodes_to_add_tabs_to: Vec<(SurfaceIndex, NodeIndex)>,
    backend: Arc<RobotBackend>,
    egui_context: Context,
}

impl TabViewer {
    fn panel_ui_context(&self) -> PanelUiContext<'_> {
        PanelUiContext {
            backend: &self.backend,
            egui_context: self.egui_context.clone(),
        }
    }
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.push_id(tab.ui_id, |ui| match &mut tab.panel {
            Ok(panel) => panel.ui(ui, self.panel_ui_context()),
            Err(error) => {
                ui.label(format!("Error loading panel: {error:#}"));
            }
        });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        match &mut tab.panel {
            Ok(panel) => format!("{panel}").into(),
            Err(error) => format!("{error}").into(),
        }
    }

    fn on_add(&mut self, surface_index: SurfaceIndex, node: NodeIndex) {
        self.nodes_to_add_tabs_to.push((surface_index, node));
    }
}

fn setup_logger() -> Result<()> {
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

fn main() -> eframe::Result<()> {
    color_eyre::install().expect("failed to install color-eyre");
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
                error!("{error:#}");
            }
        }
        Err(error) => {
            warn!("{error:#}");
        }
    }

    let configuration = Configuration::load()
        .unwrap_or_else(|error| panic!("failed to load configuration: {error}"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime");
    let runtime_handle = runtime.handle().clone();

    run_native(
        "Twix",
        NativeOptions::default(),
        Box::new(move |creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            let namespace = arguments
                .namespace
                .clone()
                .or_else(|| creation_context.storage?.get_string("namespace"))
                .unwrap_or_else(|| "/".to_string());
            let backend = runtime.block_on(RobotBackend::new(
                runtime_handle.clone(),
                arguments.router.clone(),
                namespace,
            ))?;
            Ok(Box::new(TwixApp::create(
                creation_context,
                arguments.clone(),
                runtime,
                Arc::new(backend),
                configuration,
            )))
        }),
    )
}
