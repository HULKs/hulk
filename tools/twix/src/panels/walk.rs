use std::sync::Arc;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};
use color_eyre::{
    eyre::{eyre, Error},
    Result,
};
use communication::messages::TextOrBinary;
use eframe::egui::{Response, ScrollArea, TextEdit, Ui, Widget};
use egui_plot::Plot;
use hulk_widgets::{NaoPathCompletionEdit, PathFilter};
use log::error;
use parameters::directory::Scope;
use serde_json::{json, Value};

pub struct WalkPanel {
    nao: Arc<Nao>,
    walking_engine: BufferHandle<Engine>,
}

impl Panel for WalkPanel {
    const NAME: &'static str = "Walk";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let walking_engine = nao.subscribe_value("Control.additional_outputs.walking.engine");

        Self {
            nao,
            walking_engine,
        }
    }
    fn save(&self) -> Value {
        json!({})
    }
}

impl Widget for &mut ParameterPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let engine = self.walking_

        Plot::new(ui.auto_id_with("Walk Plot"))
            .data_aspect(1.0)
            .show(ui, |plot_ui| {
                if let Some(engine) = self.walking_engine.get() {
                    if let Some(step) = engine.debug_output.step {
                        plot_ui.line(egui_plot::Line::new(egui_plot::Values::from_values_iter(
                            step.iter().enumerate().map(|(i, v)| (i as f64, *v as f64)),
                        )));
                    } else {
                        plot_ui.text("No step data available");
                    }
                } else {
                    plot_ui.text("No walking engine data available");
                }
            });
    }
}
