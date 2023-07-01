use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
    sync::Arc,
    time::SystemTime,
};

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
use egui_dock::{DockArea, NodeIndex, TabAddAlign, TabIndex, Tree};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use nao::Nao;
use panel::Panel;
use panels::{
    BehaviorSimulatorPanel, ImagePanel, ImageSegmentsPanel, LookAtPanel, ManualCalibrationPanel,
    MapPanel, ParameterPanel, PlotPanel, RemotePanel, SegmenterCalibrationPanel, TextPanel,
};
use serde_json::{from_str, to_string, Value};
use tokio::sync::mpsc;
use visuals::Visuals;

mod completion_edit;
mod image_buffer;
mod nao;
mod panel;
mod panels;
mod players_value_buffer;
mod repository_parameters;
mod twix_painter;
mod value_buffer;
pub mod visuals;

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
    let options = NativeOptions::default();
    run_native(
        "Twix",
        options,
        Box::new(|creation_context| Box::new(TwixApp::create(creation_context))),
    )
}

#[allow(clippy::large_enum_variant)]
enum SelectablePanel {
    BehaviorSimulator(BehaviorSimulatorPanel),
    Text(TextPanel),
    Plot(PlotPanel),
    Image(ImagePanel),
    ImageSegments(ImageSegmentsPanel),
    Map(MapPanel),
    Parameter(ParameterPanel),
    ManualCalibration(ManualCalibrationPanel),
    LookAt(LookAtPanel),
    SegmenterCalibration(SegmenterCalibrationPanel),
    Remote(RemotePanel),
}

impl SelectablePanel {
    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Result<SelectablePanel> {
        let name = value
            .ok_or(eyre!("Got none value"))?
            .get("_panel_type")
            .ok_or(eyre!("value has no _panel_type: {value:?}"))?
            .as_str()
            .ok_or(eyre!("_panel_type is not a string"))?;
        Self::try_from_name(name, nao, value)
    }

    fn try_from_name(name: &str, nao: Arc<Nao>, value: Option<&Value>) -> Result<SelectablePanel> {
        Ok(match name.to_lowercase().as_str() {
            "behavior simulator" => {
                SelectablePanel::BehaviorSimulator(BehaviorSimulatorPanel::new(nao, value))
            }
            "text" => SelectablePanel::Text(TextPanel::new(nao, value)),
            "plot" => SelectablePanel::Plot(PlotPanel::new(nao, value)),
            "image" => SelectablePanel::Image(ImagePanel::new(nao, value)),
            "image segments" => SelectablePanel::ImageSegments(ImageSegmentsPanel::new(nao, value)),
            "map" => SelectablePanel::Map(MapPanel::new(nao, value)),
            "parameter" => SelectablePanel::Parameter(ParameterPanel::new(nao, value)),
            "manual calibration" => {
                SelectablePanel::ManualCalibration(ManualCalibrationPanel::new(nao, value))
            }
            "look at" => SelectablePanel::LookAt(LookAtPanel::new(nao, value)),
            "segmenter calibration" => {
                SelectablePanel::SegmenterCalibration(SegmenterCalibrationPanel::new(nao, value))
            }
            "remote" => SelectablePanel::Remote(RemotePanel::new(nao, value)),
            name => bail!("unexpected panel name: {name}"),
        })
    }

    fn save(&self) -> Value {
        let mut value = match self {
            SelectablePanel::BehaviorSimulator(panel) => panel.save(),
            SelectablePanel::Text(panel) => panel.save(),
            SelectablePanel::Plot(panel) => panel.save(),
            SelectablePanel::Image(panel) => panel.save(),
            SelectablePanel::ImageSegments(panel) => panel.save(),
            SelectablePanel::Map(panel) => panel.save(),
            SelectablePanel::Parameter(panel) => panel.save(),
            SelectablePanel::ManualCalibration(panel) => panel.save(),
            SelectablePanel::LookAt(panel) => panel.save(),
            SelectablePanel::SegmenterCalibration(panel) => panel.save(),
            SelectablePanel::Remote(panel) => panel.save(),
        };
        value["_panel_type"] = Value::String(self.to_string());

        value
    }
}

impl Widget for &mut SelectablePanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        match self {
            SelectablePanel::BehaviorSimulator(panel) => panel.ui(ui),
            SelectablePanel::Text(panel) => panel.ui(ui),
            SelectablePanel::Plot(panel) => panel.ui(ui),
            SelectablePanel::Image(panel) => panel.ui(ui),
            SelectablePanel::ImageSegments(panel) => panel.ui(ui),
            SelectablePanel::Map(panel) => panel.ui(ui),
            SelectablePanel::Parameter(panel) => panel.ui(ui),
            SelectablePanel::ManualCalibration(panel) => panel.ui(ui),
            SelectablePanel::LookAt(panel) => panel.ui(ui),
            SelectablePanel::SegmenterCalibration(panel) => panel.ui(ui),
            SelectablePanel::Remote(panel) => panel.ui(ui),
        }
    }
}

impl Display for SelectablePanel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let panel_name = match self {
            SelectablePanel::BehaviorSimulator(_) => BehaviorSimulatorPanel::NAME,
            SelectablePanel::Text(_) => TextPanel::NAME,
            SelectablePanel::Plot(_) => PlotPanel::NAME,
            SelectablePanel::Image(_) => ImagePanel::NAME,
            SelectablePanel::ImageSegments(_) => ImageSegmentsPanel::NAME,
            SelectablePanel::Map(_) => MapPanel::NAME,
            SelectablePanel::Parameter(_) => ParameterPanel::NAME,
            SelectablePanel::ManualCalibration(_) => ManualCalibrationPanel::NAME,
            SelectablePanel::LookAt(_) => LookAtPanel::NAME,
            SelectablePanel::SegmenterCalibration(_) => SegmenterCalibrationPanel::NAME,
            SelectablePanel::Remote(_) => RemotePanel::NAME,
        };
        f.write_str(panel_name)
    }
}

struct TwixApp {
    nao: Arc<Nao>,
    connection_intent: bool,
    ip_address: String,
    panel_selection: String,
    last_focused_tab: (NodeIndex, TabIndex),
    tree: Tree<Tab>,
    connection_status: ConnectionStatus,
    connection_receiver: mpsc::Receiver<ConnectionStatus>,
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

        let tree: Option<Tree<Value>> = creation_context
            .storage
            .and_then(|storage| storage.get_string("tree"))
            .and_then(|string| from_str(&string).ok());

        let tree = match tree {
            Some(tree) => tree.map_tabs(|value| {
                SelectablePanel::new(nao.clone(), Some(value))
                    .unwrap()
                    .into()
            }),
            None => Tree::new(vec![SelectablePanel::Text(TextPanel::new(
                nao.clone(),
                None,
            ))
            .into()]),
        };

        let connection_status = ConnectionStatus::Disconnected {
            address: None,
            connect: false,
        };
        let connection_receiver = nao.subscribe_status_updates();

        let visual = creation_context
            .storage
            .and_then(|storage| storage.get_string("style"))
            .and_then(|theme| Visuals::from_str(&theme).ok())
            .unwrap_or(Visuals::Dark);
        visual.set_visual(&creation_context.egui_ctx);

        let panel_selection = "".to_string();
        Self {
            nao,
            connection_intent,
            ip_address: ip_address.unwrap_or_default(),
            panel_selection,
            tree,
            last_focused_tab: (0.into(), 0.into()),
            connection_status,
            connection_receiver,
            visual,
        }
    }
}

impl App for TwixApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        while let Ok(status) = self.connection_receiver.try_recv() {
            self.connection_status = status;
        }

        context.request_repaint();
        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let address_input =
                        CompletionEdit::addresses(&mut self.ip_address, 21..=37).ui(ui);
                    if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::O)) {
                        address_input.request_focus();
                        CompletionEdit::select_all(&self.ip_address, ui, address_input.id);
                    }
                    if address_input.changed() || address_input.lost_focus() {
                        self.nao.set_address(&self.ip_address);
                    }
                    let (connect_text, color) = match &self.connection_status {
                        ConnectionStatus::Disconnected { connect, .. } => (
                            "Connect",
                            if *connect {
                                Color32::RED
                            } else {
                                Color32::WHITE
                            },
                        ),
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
                        vec![
                            "Behavior Simulator".to_string(),
                            "Text".to_string(),
                            "Plot".to_string(),
                            "Image".to_string(),
                            "Image Segments".to_string(),
                            "Map".to_string(),
                            "Parameter".to_string(),
                            "Manual Calibration".to_string(),
                            "Look At".to_string(),
                            "Segmenter Calibration".to_string(),
                            "Remote".to_string(),
                        ],
                        "Panel",
                    )
                    .ui(ui);
                    if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::P)) {
                        panel_input.request_focus();
                        CompletionEdit::select_all(&self.panel_selection, ui, panel_input.id);
                    }
                    if panel_input.changed() || panel_input.lost_focus() {
                        if let Ok(panel) = SelectablePanel::try_from_name(
                            &self.panel_selection,
                            self.nao.clone(),
                            None,
                        ) {
                            if let Some(active_panel) = self.active_panel() {
                                *active_panel = panel;
                            }
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
                let tab = SelectablePanel::Text(TextPanel::new(self.nao.clone(), None));
                self.tree.push_to_focused_leaf(tab.into());
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.buttons.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer::default();
            DockArea::new(&mut self.tree)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut tab_viewer);
            for node_id in tab_viewer.nodes_to_add_tabs_to {
                let tab = SelectablePanel::Text(TextPanel::new(self.nao.clone(), None));
                let index = self.tree[node_id].tabs_count();
                self.tree[node_id].insert_tab(index.into(), tab.into());
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
        let node = self.tree.focused_leaf()?;
        if let egui_dock::Node::Leaf { active, .. } = &self.tree[node] {
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
    nodes_to_add_tabs_to: Vec<NodeIndex>,
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

    fn on_add(&mut self, node: NodeIndex) {
        self.nodes_to_add_tabs_to.push(node);
    }
}
