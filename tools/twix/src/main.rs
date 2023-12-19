use std::{
    fmt::{self, Display, Formatter},
    net::IpAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use aliveness::query_aliveness;
use color_eyre::{
    eyre::{bail, eyre},
    Result,
};

use communication::client::ConnectionStatus;
use completion_edit::CompletionEdit;
use eframe::{
    egui::{
        CentralPanel, Context, Id, Key, Layout, Modifiers, TopBottomPanel, Ui, Widget, WidgetText,
    },
    emath::Align,
    epaint::Color32,
    run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabAddAlign, TabIndex};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use log::error;
use nao::Nao;
use panel::Panel;
use panels::{
    BehaviorSimulatorPanel, ImagePanel, ImageSegmentsPanel, LookAtPanel, ManualCalibrationPanel,
    MapPanel, ParameterPanel, PlotPanel, RemotePanel, TextPanel, VisionTunerPanel,
};
use repository::{get_repository_root, Repository};
use serde_json::{from_str, to_string, Value};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use visuals::Visuals;

mod completion_edit;
mod image_buffer;
mod nao;
mod panel;
mod panels;
mod players_value_buffer;
mod repository_parameters;
mod selectable_panel_macro;
mod twix_painter;
mod value_buffer;
mod visuals;

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

    let runtime = Runtime::new().unwrap();
    if let Ok(repository_root) = runtime.block_on(get_repository_root()) {
        Repository::new(repository_root)
            .check_new_version_available(env!("CARGO_PKG_VERSION"), "tools/twix")
            .unwrap();
    }

    let options = NativeOptions::default();
    run_native(
        "Twix",
        options,
        Box::new(|creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            Box::new(TwixApp::create(creation_context))
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
    RemotePanel,
    TextPanel,
    VisionTunerPanel
);
struct TwixApp {
    nao: Arc<Nao>,
    reachable_naos: ReachableNaos,
    connection_intent: bool,
    ip_address: String,
    panel_selection: String,
    last_focused_tab: (NodeIndex, TabIndex),
    tree: DockState<Tab>,
    visual: Visuals,
}

impl TwixApp {
    fn create(creation_context: &CreationContext) -> Self {
        let ip_address = creation_context
            .storage
            .map(|storage| storage.get_string("ip_address"))
            .unwrap_or(None);

        let connection_intent = creation_context
            .storage
            .and_then(|storage| {
                storage
                    .get_string("connection_intent")
                    .map(|stored| stored == "true")
            })
            .unwrap_or(false);

        let nao = Arc::new(Nao::new(ip_address.clone(), connection_intent));

        let tree: Option<DockState<Value>> = creation_context
            .storage
            .and_then(|storage| storage.get_string("tree"))
            .and_then(|string| from_str(&string).ok());

        let tree = match tree {
            Some(tree) => tree.map_tabs(|value| {
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
        let reachable_naos = ReachableNaos::new(context.clone());
        nao.on_update(move || context.request_repaint());

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
            ip_address: ip_address.unwrap_or_default(),
            panel_selection,
            tree,
            last_focused_tab: (0.into(), 0.into()),
            visual,
        }
    }
}

impl App for TwixApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        self.reachable_naos.update();

        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let address_input = CompletionEdit::addresses(
                        &mut self.ip_address,
                        21..=37,
                        &self.reachable_naos.ips,
                    )
                    .ui(ui);
                    if address_input.gained_focus() {
                        self.reachable_naos.query_reachability();
                    }
                    if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::O)) {
                        address_input.request_focus();
                        CompletionEdit::select_all(&self.ip_address, ui, address_input.id);
                    }
                    if address_input.changed() || address_input.lost_focus() {
                        self.nao.set_address(&self.ip_address);
                    }
                    let (connect_text, color) = match self.nao.connection_status() {
                        ConnectionStatus::Disconnected { connect, .. } => {
                            if connect {
                                ("Connecting", Color32::RED)
                            } else {
                                ("Disconnected", Color32::WHITE)
                            }
                        }
                        ConnectionStatus::Connecting { .. } => ("Connecting", Color32::YELLOW),
                        ConnectionStatus::Connected { .. } => ("Connected", Color32::GREEN),
                    };
                    let connect_text = WidgetText::from(connect_text).color(color);
                    if ui
                        .checkbox(&mut self.connection_intent, connect_text)
                        .changed()
                    {
                        self.nao.set_connect(self.connection_intent);
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
                    if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::P)) {
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
            if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::T)) {
                let tab = SelectablePanel::TextPanel(TextPanel::new(self.nao.clone(), None));
                self.tree.push_to_focused_leaf(tab.into());
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.buttons.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer::default();
            DockArea::new(&mut self.tree)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut tab_viewer);
            
            for (surface_index, node_id) in tab_viewer.nodes_to_add_tabs_to {
                let tab = SelectablePanel::TextPanel(TextPanel::new(self.nao.clone(), None));
                let index = self.tree[surface_index][node_id].tabs_count();
                self.tree[surface_index][node_id].insert_tab(index.into(), tab.into());
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let tree = self.tree.map_tabs(|tab| tab.panel.save());

        storage.set_string("tree", to_string(&tree).unwrap());
        storage.set_string("ip_address", self.ip_address.clone());
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
        let (_viewport, tab) = self.tree.find_active_focused()?;
        Some(&mut tab.panel)
    }

    fn active_tab_index(&self) -> Option<(NodeIndex, TabIndex)> {
        let (surface, node) = self.tree.focused_leaf()?;
        if let egui_dock::Node::Leaf { active, .. } = &self.tree[surface][node] {
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
