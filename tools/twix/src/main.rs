use std::{
    fmt::{self, Display, Formatter},
    net::IpAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use aliveness::query_aliveness;
use clap::Parser;
use color_eyre::{
    eyre::{bail, eyre},
    Result,
};

use communication::client::Status;
use completion_edit::CompletionEdit;
use configuration::{
    keybind_plugin::{self, KeybindSystem},
    keys::KeybindAction,
    Configuration,
};
use eframe::{
    egui::{CentralPanel, Context, Id, Layout, TopBottomPanel, Ui, Widget, WidgetText},
    emath::Align,
    epaint::{Color32, Rounding},
    run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use egui_dock::{DockArea, DockState, Node, NodeIndex, Split, SurfaceIndex, TabAddAlign, TabIndex};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use log::error;
use nao::Nao;
use panel::Panel;
use panels::{
    BehaviorSimulatorPanel, EnumPlotPanel, ImageColorSelectPanel, ImagePanel, ImageSegmentsPanel,
    LookAtPanel, ManualCalibrationPanel, MapPanel, ParameterPanel, PlotPanel, RemotePanel,
    TextPanel, VisionTunerPanel,
};

use repository::{get_repository_root, Repository};
use serde_json::{from_str, to_string, Value};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use visuals::Visuals;

mod completion_edit;
mod configuration;
mod log_error;
mod nao;
mod panel;
mod panels;
mod players_buffer_handle;
mod repository_parameters;
mod selectable_panel_macro;
mod twix_painter;
mod value_buffer;
mod visuals;
mod zoom_and_pan;

#[derive(Debug, Parser)]
struct Arguments {
    /// Nao address to connect to (overrides the address saved in the configuration file)
    pub address: Option<String>,

    /// Delete the current panel setup
    #[arg(long)]
    pub clear: bool,
}

fn setup_logger() -> Result<(), InitError> {
    Dispatch::new()
        .format(|out, message, record| {
            let colors = ColoredLevelConfig::new();
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn main() -> Result<(), eframe::Error> {
    setup_logger().unwrap();
    let arguments = Arguments::parse();

    let runtime = Runtime::new().unwrap();
    if let Ok(repository_root) = runtime.block_on(get_repository_root()) {
        Repository::new(repository_root)
            .check_new_version_available(env!("CARGO_PKG_VERSION"), "tools/twix")
            .unwrap();
    }

    let configuration = Configuration::load()
        .unwrap_or_else(|error| panic!("failed to load configuration: {error}"));

    let options = NativeOptions::default();
    run_native(
        "Twix",
        options,
        Box::new(|creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            Box::new(TwixApp::create(creation_context, arguments, configuration))
        }),
    )
}

struct ReachableNaos {
    ips: Vec<IpAddr>,
    tx: UnboundedSender<Vec<IpAddr>>,
    rx: UnboundedReceiver<Vec<IpAddr>>,
    context: Context,
    runtime: Runtime,
}

impl ReachableNaos {
    pub fn new(context: Context) -> Self {
        let ips = Vec::new();
        let (tx, rx) = unbounded_channel();
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        Self {
            ips,
            tx,
            rx,
            context,
            runtime,
        }
    }

    pub fn query_reachability(&self) {
        let tx = self.tx.clone();
        let context = self.context.clone();
        self.runtime.spawn(async move {
            if let Ok(ips) = query_aliveness(Duration::from_millis(200), None).await {
                let ips = ips.into_iter().map(|(ip, _)| ip).collect();
                let _ = tx.send(ips);
                context.request_repaint();
            }
        });
    }

    pub fn update(&mut self) {
        while let Ok(ips) = self.rx.try_recv() {
            self.ips = ips;
        }
    }
}

impl_selectable_panel!(
    BehaviorSimulatorPanel,
    ImagePanel,
    ImageSegmentsPanel,
    LookAtPanel,
    ManualCalibrationPanel,
    MapPanel,
    ParameterPanel,
    PlotPanel,
    EnumPlotPanel,
    RemotePanel,
    TextPanel,
    VisionTunerPanel,
    ImageColorSelectPanel,
);
struct TwixApp {
    nao: Arc<Nao>,
    reachable_naos: ReachableNaos,
    connection_intent: bool,
    address: String,
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
    ) -> Self {
        let address = arguments
            .address
            .or_else(|| creation_context.storage?.get_string("address"))
            .unwrap_or_else(|| "localhost".to_string());

        let nao = Arc::new(Nao::new(format!("ws://{address}:1337")));

        let connection_intent = creation_context
            .storage
            .and_then(|storage| storage.get_string("connection_intent"))
            .map(|stored| stored == "true")
            .unwrap_or(false);

        if connection_intent {
            nao.connect();
        }

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
                SelectablePanel::new(nao.clone(), Some(value))
                    .unwrap()
                    .into()
            }),
            None => DockState::new(vec![SelectablePanel::TextPanel(TextPanel::new(
                nao.clone(),
                None,
            ))
            .into()]),
        };

        let context = creation_context.egui_ctx.clone();

        keybind_plugin::register(&context);
        context.set_keybinds(Arc::new(configuration.keys));

        let reachable_naos = ReachableNaos::new(context.clone());
        nao.on_change(move || context.request_repaint());

        let visual = creation_context
            .storage
            .and_then(|storage| storage.get_string("style"))
            .and_then(|theme| Visuals::from_str(&theme).ok())
            .unwrap_or(Visuals::Dark);
        visual.set_visual(&creation_context.egui_ctx);

        let panel_selection = "".to_string();

        Self {
            nao,
            reachable_naos,
            connection_intent,
            address,
            panel_selection,
            dock_state,
            last_focused_tab: (0.into(), 0.into()),
            visual,
        }
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
        self.reachable_naos.update();

        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let address_input = CompletionEdit::addresses(
                        &mut self.address,
                        21..=41,
                        &self.reachable_naos.ips,
                    )
                    .ui(ui);
                    if address_input.gained_focus() {
                        self.reachable_naos.query_reachability();
                    }
                    if context.keybind_pressed(KeybindAction::FocusAddress) {
                        address_input.request_focus();
                        CompletionEdit::select_all(&self.address, ui, address_input.id);
                    }
                    if address_input.changed() || address_input.lost_focus() {
                        let address = &self.address;
                        self.nao.set_address(format!("ws://{address}:1337"));
                    }
                    let (connect_text, color) = match self.nao.connection_status() {
                        Status::Disconnected => ("Disconnected", Color32::RED),
                        Status::Connecting => ("Connecting", Color32::YELLOW),
                        Status::Connected => ("Connected", Color32::GREEN),
                    };
                    let connect_text = WidgetText::from(connect_text).color(color);
                    if ui
                        .checkbox(&mut self.connection_intent, connect_text)
                        .changed()
                    {
                        if self.connection_intent {
                            self.nao.connect();
                        } else {
                            self.nao.disconnect();
                        }
                    }
                    if context.keybind_pressed(KeybindAction::Reconnect) {
                        self.nao.disconnect();
                        self.connection_intent = true;
                        self.nao.connect();
                    }

                    if self.active_tab_index() != Some(self.last_focused_tab) {
                        self.last_focused_tab =
                            self.active_tab_index().unwrap_or((0.into(), 0.into()));
                        if let Some(name) = self.active_panel().map(|panel| format!("{panel}")) {
                            self.panel_selection = name
                        }
                    }
                    let panel_input = CompletionEdit::new(
                        &mut self.panel_selection,
                        SelectablePanel::registered()
                            .into_iter()
                            .map(|registered| registered.into())
                            .collect(),
                        "Panel",
                    )
                    .ui(ui);
                    if context.keybind_pressed(KeybindAction::FocusPanel) {
                        panel_input.request_focus();
                        CompletionEdit::select_all(&self.panel_selection, ui, panel_input.id);
                    }
                    if panel_input.changed() || panel_input.lost_focus() {
                        match SelectablePanel::try_from_name(
                            &self.panel_selection,
                            self.nao.clone(),
                            None,
                        ) {
                            Ok(panel) => {
                                if let Some(active_panel) = self.active_panel() {
                                    *active_panel = panel;
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
                    })
                });
            })
        });
        CentralPanel::default().show(context, |ui| {
            if context.keybind_pressed(KeybindAction::OpenSplit) {
                let tab = SelectablePanel::TextPanel(TextPanel::new(self.nao.clone(), None));
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
                let tab = SelectablePanel::TextPanel(TextPanel::new(self.nao.clone(), None));
                self.dock_state.push_to_focused_leaf(tab.into());
            }

            if context.keybind_pressed(KeybindAction::FocusLeft) {
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    self.focus_left(node_id, surface_index);
                }
            }
            if context.keybind_pressed(KeybindAction::FocusBelow) {
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    self.focus_below(node_id, surface_index);
                }
            }
            if context.keybind_pressed(KeybindAction::FocusAbove) {
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    self.focus_above(node_id, surface_index);
                }
            }
            if context.keybind_pressed(KeybindAction::FocusRight) {
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    self.focus_right(node_id, surface_index);
                }
            }

            if context.keybind_pressed(KeybindAction::DuplicateTab) {
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    let new_tab = &tab.panel.save();
                    self.dock_state.push_to_focused_leaf(Tab::from(
                        SelectablePanel::new(self.nao.clone(), Some(new_tab)).unwrap(),
                    ));
                }
            }

            if context.keybind_pressed(KeybindAction::CloseTab) {
                if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                    let active_node = &mut self.dock_state[surface_index][node_id];
                    if let Node::Leaf { active, tabs, .. } = active_node {
                        if !tabs.is_empty() {
                            tabs.remove(active.0);

                            active.0 = active.0.saturating_sub(1);

                            if tabs.is_empty() && node_id != NodeIndex(0) {
                                self.dock_state[surface_index].remove_leaf(node_id);
                            }
                        }
                    }
                }
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.buttons.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer::default();
            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut tab_viewer);

            for (surface_index, node_id) in tab_viewer.nodes_to_add_tabs_to {
                let tab = SelectablePanel::TextPanel(TextPanel::new(self.nao.clone(), None));
                let index = self.dock_state[surface_index][node_id].tabs_count();
                self.dock_state[surface_index][node_id].insert_tab(index.into(), tab.into());
                self.dock_state
                    .set_focused_node_and_surface((surface_index, node_id));
            }

            if let Some((surface_index, node_id)) = self.dock_state.focused_leaf() {
                let node = &self.dock_state[surface_index][node_id];
                let rect = node.rect().unwrap();
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(4.0),
                    ui.style().visuals.widgets.active.bg_stroke,
                );
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let dock_state = self.dock_state.map_tabs(|tab| tab.panel.save());

        storage.set_string("dock_state", to_string(&dock_state).unwrap());
        storage.set_string("address", self.address.clone());
        storage.set_string(
            "connection_intent",
            if self.connection_intent {
                "true"
            } else {
                "false"
            }
            .to_string(),
        );
        storage.set_string("style", self.visual.to_string());
    }
}

impl TwixApp {
    fn active_panel(&mut self) -> Option<&mut SelectablePanel> {
        let (_viewport, tab) = self.dock_state.find_active_focused()?;
        Some(&mut tab.panel)
    }

    fn active_tab_index(&self) -> Option<(NodeIndex, TabIndex)> {
        let (surface, node) = self.dock_state.focused_leaf()?;
        if let Node::Leaf { active, .. } = &self.dock_state[surface][node] {
            Some((node, *active))
        } else {
            None
        }
    }
}

struct Tab {
    id: Id,
    panel: SelectablePanel,
}

impl From<SelectablePanel> for Tab {
    fn from(panel: SelectablePanel) -> Self {
        Self {
            id: Id::new(SystemTime::now()),
            panel,
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
        tab.panel.ui(ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        format!("{}", tab.panel).into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        tab.id
    }

    fn on_add(&mut self, surface_index: SurfaceIndex, node: NodeIndex) {
        self.nodes_to_add_tabs_to.push((surface_index, node));
    }
}
