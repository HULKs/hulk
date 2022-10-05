use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use anyhow::Result;

use completion_edit::CompletionEdit;
use eframe::{
    egui::{CentralPanel, Context, Key, Modifiers, TopBottomPanel, Visuals, Widget},
    run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use log::warn;
use nao::Nao;
use panel::Panel;
use panels::{ImagePanel, ImageSegmentsPanel, MapPanel, ParameterPanel, PlotPanel, TextPanel};

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
    fn save(&mut self, storage: &mut dyn Storage) {
        match self {
            SelectablePanel::Text(panel) => panel.save(storage),
            SelectablePanel::Plot(panel) => panel.save(storage),
            SelectablePanel::Image(panel) => panel.save(storage),
            SelectablePanel::ImageSegments(panel) => panel.save(storage),
            SelectablePanel::Map(panel) => panel.save(storage),
            SelectablePanel::Parameter(panel) => panel.save(storage),
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
    active_panel: SelectablePanel,
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

        let (panel_selection, active_panel) = match creation_context
            .storage
            .and_then(|storage| storage.get_string("selected_panel"))
        {
            Some(stored_panel) => {
                let panel = match stored_panel.as_str() {
                    "Text" => {
                        SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage))
                    }
                    "Plot" => {
                        SelectablePanel::Plot(PlotPanel::new(nao.clone(), creation_context.storage))
                    }
                    "Image" => SelectablePanel::Image(ImagePanel::new(
                        nao.clone(),
                        creation_context.storage,
                    )),
                    "Image Segments" => SelectablePanel::ImageSegments(ImageSegmentsPanel::new(
                        nao.clone(),
                        creation_context.storage,
                    )),
                    "Map" => {
                        SelectablePanel::Map(MapPanel::new(nao.clone(), creation_context.storage))
                    }
                    "Parameter" => SelectablePanel::Parameter(ParameterPanel::new(
                        nao.clone(),
                        creation_context.storage,
                    )),
                    name => {
                        warn!("Unknown panel stored in persistent storage: {name}");
                        SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage))
                    }
                };
                (stored_panel, panel)
            }
            None => (
                "Text".to_string(),
                SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage)),
            ),
        };

        let mut style = (*creation_context.egui_ctx.style()).clone();
        style.visuals = Visuals::dark();
        creation_context.egui_ctx.set_style(style);
        Self {
            nao,
            connection_intent,
            ip_address: ip_address.unwrap_or_default(),
            panel_selection,
            active_panel,
        }
    }
}

fn ip_to_socket_address(ip_address: &str) -> String {
    format!("ws://{}:1337", ip_address)
}

impl App for TwixApp {
    fn update(&mut self, context: &Context, frame: &mut Frame) {
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
                if ui
                    .checkbox(&mut self.connection_intent, "Connect")
                    .changed()
                {
                    self.nao.set_connect(self.connection_intent);
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
                    match self.panel_selection.to_lowercase().as_str() {
                        "text" => {
                            self.active_panel = SelectablePanel::Text(TextPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ))
                        }
                        "plot" => {
                            self.active_panel = SelectablePanel::Plot(PlotPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ));
                        }
                        "image" => {
                            self.active_panel = SelectablePanel::Image(ImagePanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ))
                        }
                        "image segments" => {
                            self.active_panel = SelectablePanel::ImageSegments(
                                ImageSegmentsPanel::new(self.nao.clone(), frame.storage()),
                            )
                        }
                        "map" => {
                            self.active_panel = SelectablePanel::Map(MapPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ))
                        }
                        "parameter" => {
                            self.active_panel = SelectablePanel::Parameter(ParameterPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ))
                        }
                        _ => {}
                    }
                }
            })
        });
        CentralPanel::default().show(context, |ui| match &mut self.active_panel {
            SelectablePanel::Text(panel) => panel.ui(ui),
            SelectablePanel::Plot(panel) => panel.ui(ui),
            SelectablePanel::Image(panel) => panel.ui(ui),
            SelectablePanel::ImageSegments(panel) => panel.ui(ui),
            SelectablePanel::Map(panel) => panel.ui(ui),
            SelectablePanel::Parameter(panel) => panel.ui(ui),
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
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
        storage.set_string("selected_panel", self.active_panel.to_string());
        self.active_panel.save(storage);
    }
}
