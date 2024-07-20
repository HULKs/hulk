use std::{ops::RangeInclusive, sync::Arc};

use color_eyre::Result;
use communication::messages::TextOrBinary;
use eframe::{
    egui::{Grid, Response, Slider, Ui, Widget},
    emath::Numeric,
};
use log::error;
use serde::Serialize;
use serde_json::{to_value, Value};

use types::{field_color::FieldColorParameters, image_segments::Direction};

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

use super::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    cycler: VisionCycler,
    horizontal_edge_threshold: BufferHandle<u8>,
    vertical_edge_threshold: BufferHandle<u8>,
    field_color_detection: BufferHandle<FieldColorParameters>,
}

impl VisionTunerPanel {
    fn edge_threshold_slider(&mut self, ui: &mut Ui, direction: Direction) -> Result<()> {
        let (value_buffer, parameter_name) = match direction {
            Direction::Horizontal => (&self.horizontal_edge_threshold, "horizontal_edge_threshold"),
            Direction::Vertical => (&self.vertical_edge_threshold, "vertical_edge_threshold"),
        };

        let Some(mut edge_threshold) = value_buffer.get_last_value()? else {
            return Ok(());
        };

        let slider = ui.add(Slider::new(&mut edge_threshold, 0..=255).text(parameter_name));
        if slider.changed() {
            let cycler = self.cycler.as_snake_case_path();

            self.nao.write(
                format!("parameters.image_segmenter.{cycler}.{parameter_name}"),
                TextOrBinary::Text(to_value(edge_threshold).unwrap()),
            );
        }

        Ok(())
    }

    fn row<T: Numeric + Serialize>(
        &mut self,
        ui: &mut Ui,
        cycler: &str,
        parameter: &'static str,
        value: RangeInclusive<T>,
        range: RangeInclusive<T>,
    ) {
        ui.label(parameter);
        let mut start = *value.start();
        let slider = ui.add(Slider::new(&mut start, range.clone()));
        if slider.changed() {
            self.nao.write(
                format!("parameters.field_color_detection.{cycler}.{parameter}.start"),
                TextOrBinary::Text(to_value(start).unwrap()),
            );
        }
        let mut end = *value.end();
        let slider = ui.add(Slider::new(&mut end, range));
        if slider.changed() {
            self.nao.write(
                format!("parameters.field_color_detection.{cycler}.{parameter}.end"),
                TextOrBinary::Text(to_value(end).unwrap()),
            );
        }
        ui.end_row();
    }
}

impl Panel for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let cycler = VisionCycler::Top;

        let cycler_path = cycler.as_snake_case_path();
        let horizontal_edge_threshold = nao.subscribe_value(format!(
            "parameters.image_segmenter.{cycler_path}.horizontal_edge_threshold",
        ));
        let vertical_edge_threshold = nao.subscribe_value(format!(
            "parameters.image_segmenter.{cycler_path}.vertical_edge_threshold",
        ));
        let field_color_detection =
            nao.subscribe_value(format!("parameters.field_color_detection.{cycler_path}",));

        Self {
            nao,
            cycler,
            horizontal_edge_threshold,
            vertical_edge_threshold,
            field_color_detection,
        }
    }
}

impl Widget for &mut VisionTunerPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.style_mut().spacing.slider_width = (ui.available_size().x - 250.0) / 2.0;
        let layout = ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
                if cycler_selector.ui(ui).changed() {
                    self.resubscribe();
                }
            });
            ui.separator();
            let cycler = self.cycler.as_snake_case_path();

            self.edge_threshold_slider(ui, Direction::Vertical)?;
            self.edge_threshold_slider(ui, Direction::Horizontal)?;

            let Some(field_color_detection) = self.field_color_detection.get_last_value()? else {
                return Ok(());
            };

            Grid::new("field_color_sliders").show(ui, |ui| {
                self.row(
                    ui,
                    &cycler,
                    "luminance",
                    field_color_detection.luminance,
                    0..=255,
                );
                self.row(
                    ui,
                    &cycler,
                    "green_luminance",
                    field_color_detection.green_luminance,
                    0..=255,
                );
                self.row(
                    ui,
                    &cycler,
                    "red_chromaticity",
                    field_color_detection.red_chromaticity,
                    0.0..=1.0,
                );
                self.row(
                    ui,
                    &cycler,
                    "green_chromaticity",
                    field_color_detection.green_chromaticity,
                    0.0..=1.0,
                );
                self.row(
                    ui,
                    &cycler,
                    "blue_chromaticity",
                    field_color_detection.blue_chromaticity,
                    0.0..=1.0,
                );
                self.row(ui, &cycler, "hue", field_color_detection.hue, 0..=360);
                self.row(
                    ui,
                    &cycler,
                    "saturation",
                    field_color_detection.saturation,
                    0..=255,
                );
            });

            Ok::<(), color_eyre::Report>(())
        });
        if let Err(error) = layout.inner {
            error!("failed to render vision tuner panel: {error}");
        }
        layout.response
    }
}

impl VisionTunerPanel {
    fn resubscribe(&mut self) {
        let cycler_path = self.cycler.as_snake_case_path();
        self.vertical_edge_threshold = self.nao.subscribe_value(format!(
            "parameters.image_segmenter.{cycler_path}.vertical_edge_threshold"
        ));
        self.field_color_detection = self
            .nao
            .subscribe_value(format!("parameters.field_color_detection.{cycler_path}"));
    }
}
