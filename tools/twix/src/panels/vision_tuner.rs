use std::sync::Arc;

use color_eyre::{eyre::OptionExt, Result};
use communication::messages::TextOrBinary;
use eframe::egui::{Response, Slider, Ui, Widget};
use log::error;
use parameters::directory::Scope;
use serde_json::{to_value, Value};

use types::image_segments::Direction;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};

use super::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    cycler: VisionCycler,
    horizontal_edge_threshold: BufferHandle<u8>,
    vertical_edge_threshold: BufferHandle<u8>,
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

    fn save_image_segmenter_parameters(&self, scope: Scope) -> Result<()> {
        let cycler = self.cycler.as_snake_case_path();

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

        self.nao.store_parameters(
            &format!("image_segmenter.{cycler}.horizontal_edge_threshold"),
            horizontal_edge_threshold_value,
            scope,
        )?;
        self.nao.store_parameters(
            &format!("image_segmenter.{cycler}.vertical_edge_threshold"),
            vertical_edge_threshold_value,
            scope,
        )?;

        Ok(())
    }

    fn save(&self, scope: Scope) -> Result<()> {
        self.save_image_segmenter_parameters(scope)?;

        Ok(())
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

        Self {
            nao,
            cycler,
            horizontal_edge_threshold,
            vertical_edge_threshold,
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
                if ui.button("Save to current location").clicked() {
                    self.save(Scope::current_location()).log_err();
                }
            });
            ui.separator();

            self.edge_threshold_slider(ui, Direction::Vertical)?;
            self.edge_threshold_slider(ui, Direction::Horizontal)?;

            Ok::<(), color_eyre::Report>(())
        });
        if let Err(error) = layout.inner {
            error!("vision tuner panel: {error}");
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
    }
}
