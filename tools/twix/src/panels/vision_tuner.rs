use std::{ops::RangeInclusive, sync::Arc};

use color_eyre::{Result, eyre::OptionExt};
use communication::messages::TextOrBinary;
use eframe::{
    egui::{Grid, Response, Slider, Ui, Widget},
    emath::Numeric,
};
use log::error;
use parameters::directory::Scope;
use serde::Serialize;
use serde_json::to_value;

use types::{field_color::FieldColorParameters, image_segments::Direction};

use crate::{
    log_error::LogError,
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    value_buffer::BufferHandle,
};

pub struct VisionTunerPanel {
    robot: Arc<Robot>,
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
            self.robot.write(
                format!("parameters.image_segmenter.vision.{parameter_name}"),
                TextOrBinary::Text(to_value(edge_threshold).unwrap()),
            );
        }

        Ok(())
    }

    fn row<T: Numeric + Serialize>(
        &mut self,
        ui: &mut Ui,
        parameter: &'static str,
        value: RangeInclusive<T>,
        range: RangeInclusive<T>,
    ) {
        ui.label(parameter);
        let mut start = *value.start();
        let slider = ui.add(Slider::new(&mut start, range.clone()));
        if slider.changed() {
            self.robot.write(
                format!("parameters.field_color_detection.{parameter}.start"),
                TextOrBinary::Text(to_value(start).unwrap()),
            );
        }
        let mut end = *value.end();
        let slider = ui.add(Slider::new(&mut end, range));
        if slider.changed() {
            self.robot.write(
                format!("parameters.field_color_detection.{parameter}.end"),
                TextOrBinary::Text(to_value(end).unwrap()),
            );
        }
        ui.end_row();
    }

    fn save_field_color_parameters(&self, scope: Scope) -> Result<()> {
        let parameters = self
            .field_color_detection
            .get_last_value()?
            .ok_or_eyre("unable to retrieve parameters, nothing was saved.")?;

        let value = to_value(parameters).unwrap();

        self.robot
            .store_parameters("field_color_detection", value, scope)?;

        Ok(())
    }

    fn save_image_segmenter_parameters(&self, scope: Scope) -> Result<()> {
        let horizontal_edge_threshold = self
            .horizontal_edge_threshold
            .get_last_value()?
            .ok_or_eyre("unable to retrieve horizontal_edge_threshold, nothing was saved.")?;
        let vertical_edge_threshold = self
            .vertical_edge_threshold
            .get_last_value()?
            .ok_or_eyre("unable to retrieve vertical_edge_threshold, nothing was saved.")?;

        let horizontal_edge_threshold_value = to_value(horizontal_edge_threshold).unwrap();
        let vertical_edge_threshold_value = to_value(vertical_edge_threshold).unwrap();

        self.robot.store_parameters(
            "image_segmenter.vision.horizontal_edge_threshold",
            horizontal_edge_threshold_value,
            scope,
        )?;
        self.robot.store_parameters(
            "image_segmenter.vision.vertical_edge_threshold",
            vertical_edge_threshold_value,
            scope,
        )?;

        Ok(())
    }

    fn save(&self, scope: Scope) -> Result<()> {
        self.save_field_color_parameters(scope)?;
        self.save_image_segmenter_parameters(scope)?;

        Ok(())
    }
}

impl<'a> Panel<'a> for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(context: PanelCreationContext) -> Self {
        let horizontal_edge_threshold = context
            .robot
            .subscribe_value("parameters.image_segmenter.vision.horizontal_edge_threshold");
        let vertical_edge_threshold = context
            .robot
            .subscribe_value("parameters.image_segmenter.vision.vertical_edge_threshold");
        let field_color_detection = context
            .robot
            .subscribe_value("parameters.field_color_detection");

        Self {
            robot: context.robot,
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
                if ui.button("Save to current location").clicked() {
                    self.save(Scope::current_location()).log_err();
                }
            });
            ui.separator();

            self.edge_threshold_slider(ui, Direction::Vertical)?;
            self.edge_threshold_slider(ui, Direction::Horizontal)?;

            let Some(field_color_detection) = self.field_color_detection.get_last_value()? else {
                return Ok(());
            };

            Grid::new("field_color_sliders").show(ui, |ui| {
                self.row(ui, "luminance", field_color_detection.luminance, 0..=255);
                self.row(
                    ui,
                    "green_luminance",
                    field_color_detection.green_luminance,
                    0..=255,
                );
                self.row(
                    ui,
                    "red_chromaticity",
                    field_color_detection.red_chromaticity,
                    0.0..=1.0,
                );
                self.row(
                    ui,
                    "green_chromaticity",
                    field_color_detection.green_chromaticity,
                    0.0..=1.0,
                );
                self.row(
                    ui,
                    "blue_chromaticity",
                    field_color_detection.blue_chromaticity,
                    0.0..=1.0,
                );
                self.row(ui, "hue", field_color_detection.hue, 0..=360);
                self.row(ui, "saturation", field_color_detection.saturation, 0..=255);
            });

            Ok::<(), color_eyre::Report>(())
        });
        if let Err(error) = layout.inner {
            error!("vision tuner panel: {error}");
        }
        layout.response
    }
}
