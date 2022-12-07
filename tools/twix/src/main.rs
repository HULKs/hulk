use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use anyhow::Result;

use communication::ConnectionStatus;
use completion_edit::CompletionEdit;
use eframe::{
    egui::{
        CentralPanel, Context, Key, Modifiers, TopBottomPanel, Ui, Visuals, Widget, WidgetText,
    },
    epaint::Color32,
    run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use egui_dock::{DockArea, NodeIndex, TabAddAlign, TabIndex, Tree};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use nao::Nao;
use panel::Panel;
use panels::{ImagePanel, ImageSegmentsPanel, MapPanel, ParameterPanel, PlotPanel, TextPanel};
use serde_json::{from_str, to_string, Value};
use tokio::sync::mpsc;

mod completion_edit;
mod image_buffer;
mod nao;
mod panel;
mod panels;
mod twix_painter;
mod value_buffer;

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

fn main() {
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
    Text(TextPanel),
    Plot(PlotPanel),
    Image(ImagePanel),
    ImageSegments(ImageSegmentsPanel),
    Map(MapPanel),
    Parameter(ParameterPanel),
}

impl SelectablePanel {
    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Option<SelectablePanel> {
        let name = value?.get("_panel_type")?.as_str()?;
        Self::try_from_name(name, nao, value)
    }

    fn try_from_name(name: &str, nao: Arc<Nao>, value: Option<&Value>) -> Option<SelectablePanel> {
        Some(match name.to_lowercase().as_str() {
            "text" => SelectablePanel::Text(TextPanel::new(nao, value)),
            "plot" => SelectablePanel::Plot(PlotPanel::new(nao, value)),
            "image" => SelectablePanel::Image(ImagePanel::new(nao, value)),
            "image segments" => SelectablePanel::ImageSegments(ImageSegmentsPanel::new(nao, value)),
            "map" => SelectablePanel::Map(MapPanel::new(nao, value)),
            "parameter" => SelectablePanel::Parameter(ParameterPanel::new(nao, value)),
            _ => return None,
        })
    }

    fn save(&self) -> Value {
        let mut value = match self {
            SelectablePanel::Text(panel) => panel.save(),
            SelectablePanel::Plot(panel) => panel.save(),
            SelectablePanel::Image(panel) => panel.save(),
            SelectablePanel::ImageSegments(panel) => panel.save(),
            SelectablePanel::Map(panel) => panel.save(),
            SelectablePanel::Parameter(panel) => panel.save(),
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
            SelectablePanel::Image(panel) => panel.ui(ui),
            SelectablePanel::ImageSegments(panel) => panel.ui(ui),
            SelectablePanel::Map(panel) => panel.ui(ui),
            SelectablePanel::Parameter(panel) => panel.ui(ui),
        }
    }
}

impl Display for SelectablePanel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let panel_name = match self {
            SelectablePanel::Text(_) => TextPanel::NAME,
            SelectablePanel::Plot(_) => PlotPanel::NAME,
            SelectablePanel::Image(_) => ImagePanel::NAME,
            SelectablePanel::ImageSegments(_) => ImageSegmentsPanel::NAME,
            SelectablePanel::Map(_) => MapPanel::NAME,
            SelectablePanel::Parameter(_) => ParameterPanel::NAME,
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
    tree: Tree<SelectablePanel>,
    connection_status: ConnectionStatus,
    connection_receiver: mpsc::Receiver<ConnectionStatus>,
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

        let nao = Arc::new(Nao::new(
            ip_address.as_ref().map(|ip| ip_to_socket_address(ip)),
            connection_intent,
        ));

        let tree: Option<Tree<Value>> = creation_context
            .storage
            .and_then(|storage| storage.get_string("tree"))
            .and_then(|string| from_str(&string).ok());

        let tree = match tree {
            Some(tree) => {
                tree.map_tabs(|value| SelectablePanel::new(nao.clone(), Some(value)).unwrap())
            }
            None => Tree::new(vec![SelectablePanel::Text(TextPanel::new(
                nao.clone(),
                None,
            ))]),
        };

        let connection_status = ConnectionStatus::Disconnected {
            address: None,
            connect: false,
        };
        let connection_receiver = nao.subscribe_status_updates();

        let mut style = (*creation_context.egui_ctx.style()).clone();
        style.visuals = Visuals::dark();
        creation_context.egui_ctx.set_style(style);
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
        }
    }
}

fn ip_to_socket_address(ip_address: &str) -> String {
    format!("ws://{}:1337", ip_address)
}

impl App for TwixApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        while let Ok(status) = self.connection_receiver.try_recv() {
            self.connection_status = status;
        }

        context.request_repaint();
        TopBottomPanel::top("top_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                let address_input = CompletionEdit::addresses(&mut self.ip_address, 21..33).ui(ui);
                if ui.input_mut().consume_key(Modifiers::CTRL, Key::O) {
                    address_input.request_focus();
                    CompletionEdit::select_all(&self.ip_address, ui, address_input.id);
                }
                if address_input.changed() || address_input.lost_focus() {
                    self.nao.set_address(ip_to_socket_address(&self.ip_address));
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
                    self.last_focused_tab = self.active_tab_index().unwrap_or((0.into(), 0.into()));
                    if let Some(name) = self.active_panel().map(|panel| format!("{panel}")) {
                        self.panel_selection = name
                    }
                }
                let panel_input = CompletionEdit::new(
                    &mut self.panel_selection,
                    vec![
                        "Text".to_string(),
                        "Plot".to_string(),
                        "Image".to_string(),
                        "Image Segments".to_string(),
                        "Map".to_string(),
                        "Parameter".to_string(),
                    ],
                )
                .ui(ui);
                if ui.input_mut().consume_key(Modifiers::CTRL, Key::P) {
                    panel_input.request_focus();
                    CompletionEdit::select_all(&self.panel_selection, ui, panel_input.id);
                }
                if panel_input.changed() || panel_input.lost_focus() {
                    if let Some(panel) = SelectablePanel::try_from_name(
                        &self.panel_selection,
                        self.nao.clone(),
                        None,
                    ) {
                        if let Some(active_panel) = self.active_panel() {
                            *active_panel = panel;
                        }
                    }
                }
            })
        });
        CentralPanel::default().show(context, |ui| {
            if ui.input_mut().consume_key(Modifiers::CTRL, Key::T) {
                let tab = SelectablePanel::Text(TextPanel::new(self.nao.clone(), None));
                self.tree.push_to_focused_leaf(tab);
            }

            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.show_add_buttons = true;
            style.add_tab_align = TabAddAlign::Left;
            let mut tab_viewer = TabViewer::default();
            DockArea::new(&mut self.tree)
                .style(style)
                .show_inside(ui, &mut tab_viewer);
            for node_id in tab_viewer.nodes_to_add_tabs_to {
                let tab = SelectablePanel::Text(TextPanel::new(self.nao.clone(), None));
                let index = self.tree[node_id].tabs_count();
                self.tree[node_id].insert_tab(index.into(), tab);
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let tree = self.tree.map_tabs(|panel| panel.save());

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
    }
}

impl TwixApp {
    fn active_panel(&mut self) -> Option<&mut SelectablePanel> {
        let (_viewport, tab) = self.tree.find_active_focused()?;
        Some(tab)
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

#[derive(Default)]
struct TabViewer {
    nodes_to_add_tabs_to: Vec<NodeIndex>,
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = SelectablePanel;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        format!("{tab}").into()
    }

    fn on_add(&mut self, node: NodeIndex) {
        self.nodes_to_add_tabs_to.push(node);
    }
}
