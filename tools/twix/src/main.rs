use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use anyhow::Result;

use eframe::{
    egui::{CentralPanel, ComboBox, Context, TextEdit, TopBottomPanel, Visuals, Widget},
    run_native, App, CreationContext, Frame, NativeOptions, Storage,
};
use fern::{colors::ColoredLevelConfig, Dispatch, InitError};

use log::warn;
use nao::Nao;
use panel::Panel;
use panels::{ImageSegmentsPanel, MapPanel, ParameterPanel, PlotPanel, TextPanel};

mod completion_edit;
mod nao;
mod panel;
mod panels;
mod twix_paint;
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

enum SelectablePanel {
    Plot(PlotPanel),
    Text(TextPanel),
    ImageSegments(ImageSegmentsPanel),
    Map(MapPanel),
    Parameter(ParameterPanel),
}

impl SelectablePanel {
    fn save(&mut self, storage: &mut dyn Storage) {
        match self {
            SelectablePanel::Plot(panel) => panel.save(storage),
            SelectablePanel::Text(panel) => panel.save(storage),
            SelectablePanel::ImageSegments(panel) => panel.save(storage),
            SelectablePanel::Map(panel) => panel.save(storage),
            SelectablePanel::Parameter(panel) => panel.save(storage),
        }
    }
}

impl Display for SelectablePanel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let panel_name = match self {
            SelectablePanel::Plot(_) => PlotPanel::NAME,
            SelectablePanel::Text(_) => TextPanel::NAME,
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
    selected_panel: SelectablePanel,
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

        let selected_panel = match creation_context
            .storage
            .and_then(|storage| storage.get_string("selected_panel"))
        {
            Some(stored_panel) => match stored_panel.as_str() {
                "Text" => {
                    SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage))
                }
                "Plot" => {
                    SelectablePanel::Plot(PlotPanel::new(nao.clone(), creation_context.storage))
                }
                "Image Segments" => SelectablePanel::ImageSegments(ImageSegmentsPanel::new(
                    nao.clone(),
                    creation_context.storage,
                )),
                "Map" => SelectablePanel::Map(MapPanel::new(nao.clone(), creation_context.storage)),
                "Parameter" => SelectablePanel::Parameter(ParameterPanel::new(
                    nao.clone(),
                    creation_context.storage,
                )),
                name => {
                    warn!("Unknown panel stored in persistent storage: {name}");
                    SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage))
                }
            },
            None => SelectablePanel::Text(TextPanel::new(nao.clone(), creation_context.storage)),
        };

        let mut style = (*creation_context.egui_ctx.style()).clone();
        style.visuals = Visuals::dark();
        creation_context.egui_ctx.set_style(style);
        Self {
            nao,
            connection_intent,
            ip_address: ip_address.unwrap_or_default(),
            selected_panel,
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
                if TextEdit::singleline(&mut self.ip_address)
                    .hint_text("Address")
                    .ui(ui)
                    .lost_focus()
                {
                    self.nao.set_address(ip_to_socket_address(&self.ip_address));
                }
                if ui
                    .checkbox(&mut self.connection_intent, "Connect")
                    .changed()
                {
                    self.nao.set_connect(self.connection_intent);
                }
                ComboBox::from_label("Panel")
                    .selected_text(self.selected_panel.to_string())
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                matches!(self.selected_panel, SelectablePanel::Text(_)),
                                "Text",
                            )
                            .clicked()
                        {
                            self.selected_panel = SelectablePanel::Text(TextPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ));
                        }
                        if ui
                            .selectable_label(
                                matches!(self.selected_panel, SelectablePanel::Plot(_)),
                                "Plot",
                            )
                            .clicked()
                        {
                            self.selected_panel = SelectablePanel::Plot(PlotPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ));
                        }
                        if ui
                            .selectable_label(
                                matches!(self.selected_panel, SelectablePanel::Map(_)),
                                "Map",
                            )
                            .clicked()
                        {
                            self.selected_panel = SelectablePanel::Map(MapPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ));
                        }
                        if ui
                            .selectable_label(
                                matches!(self.selected_panel, SelectablePanel::ImageSegments(_)),
                                "Image Segments",
                            )
                            .clicked()
                        {
                            self.selected_panel = SelectablePanel::ImageSegments(
                                ImageSegmentsPanel::new(self.nao.clone(), frame.storage()),
                            );
                        }
                        if ui
                            .selectable_label(
                                matches!(self.selected_panel, SelectablePanel::Parameter(_)),
                                "Parameter",
                            )
                            .clicked()
                        {
                            self.selected_panel = SelectablePanel::Parameter(ParameterPanel::new(
                                self.nao.clone(),
                                frame.storage(),
                            ));
                        }
                    });
            })
        });
        CentralPanel::default().show(context, |ui| match &mut self.selected_panel {
            SelectablePanel::Plot(panel) => panel.ui(ui),
            SelectablePanel::Text(panel) => panel.ui(ui),
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
        storage.set_string("selected_panel", self.selected_panel.to_string());
        self.selected_panel.save(storage);
    }
}
