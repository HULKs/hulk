use std::{ops::RangeInclusive, sync::Arc};

use color_eyre::Result;
use eframe::{
    egui::{ComboBox, Response, Slider, Ui, Widget, WidgetText},
    emath::Numeric,
};
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};

use communication::client::Cycler;

use crate::{
    nao::Nao, panel::Panel, repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    repository_parameters: Result<RepositoryParameters>,
    cycler: Cycler,
    vertical_edge_threshold: ValueBuffer,
    red_chromaticity_threshold: ValueBuffer,
    blue_chromaticity_threshold: ValueBuffer,
    green_chromaticity_threshold: ValueBuffer,
    green_luminance_threshold: ValueBuffer,
    luminance_threshold: ValueBuffer,
}

impl Panel for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let cycler = Cycler::VisionTop;

        let vertical_edge_threshold =
            nao.subscribe_parameter(get_vertical_edge_threshold_path(cycler));
        let red_chromaticity_threshold =
            nao.subscribe_parameter(get_red_chromaticity_threshold_path(cycler));
        let blue_chromaticity_threshold =
            nao.subscribe_parameter(get_blue_chromaticity_threshold_path(cycler));
        let green_chromaticity_threshold =
            nao.subscribe_parameter(get_green_chromaticity_threshold_path(cycler));
        let green_luminance_threshold =
            nao.subscribe_parameter(get_green_luminance_threshold_path(cycler));
        let luminance_threshold = nao.subscribe_parameter(get_luminance_threshold_path(cycler));

        Self {
            nao,
            repository_parameters: RepositoryParameters::try_new(),
            cycler,
            vertical_edge_threshold,
            red_chromaticity_threshold,
            blue_chromaticity_threshold,
            green_chromaticity_threshold,
            green_luminance_threshold,
            luminance_threshold,
        }
    }
}

impl Widget for &mut VisionTunerPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.draw(ui) {
            Ok(response) => response,
            Err(error) => ui.label(format!("{error:#}")),
        }
    }
}

impl VisionTunerPanel {
    fn draw(&mut self, ui: &mut Ui) -> Result<Response> {
        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        self.add_selector_row(ui);
        draw_parameter_slider(
            ui,
            &self.vertical_edge_threshold,
            "vertical_edge_threshold",
            0..=255,
        )?;
        draw_parameter_slider(
            ui,
            &self.red_chromaticity_threshold,
            "red_chromaticity_threshold",
            0.0..=1.0,
        )?;
        draw_parameter_slider(
            ui,
            &self.blue_chromaticity_threshold,
            "blue_chromaticity_threshold",
            0.0..=1.0,
        )?;
        draw_parameter_slider(
            ui,
            &self.green_chromaticity_threshold,
            "green_chromaticity_threshold",
            0.0..=1.0,
        )?;
        draw_parameter_slider(
            ui,
            &self.green_luminance_threshold,
            "green_luminance_threshold",
            0..=255,
        )?;
        let response = draw_parameter_slider(
            ui,
            &self.luminance_threshold,
            "luminance_threshold",
            0..=255,
        )?;
        Ok(response)
    }

    fn add_selector_row(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            self.add_vision_cycler_selector(ui);
            match &self.repository_parameters {
                Ok(repository_parameters) => {
                    if ui
                        .button("Save all parameters of this cycler to disk")
                        .clicked()
                    {
                        if let Some(address) = self.nao.get_address() {
                            let vertical_edge_threshold =
                                self.vertical_edge_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_vertical_edge_threshold_path(self.cycler),
                                vertical_edge_threshold,
                            );
                            let red_chromaticity_threshold =
                                self.red_chromaticity_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_red_chromaticity_threshold_path(self.cycler),
                                red_chromaticity_threshold,
                            );
                            let blue_chromaticity_threshold =
                                self.blue_chromaticity_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_blue_chromaticity_threshold_path(self.cycler),
                                blue_chromaticity_threshold,
                            );
                            let green_chromaticity_threshold =
                                self.green_chromaticity_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_green_chromaticity_threshold_path(self.cycler),
                                green_chromaticity_threshold,
                            );
                            let green_luminance_threshold =
                                self.green_luminance_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_green_luminance_threshold_path(self.cycler),
                                green_luminance_threshold,
                            );
                            let luminance_threshold =
                                self.luminance_threshold.get_latest().unwrap();
                            repository_parameters.write(
                                &address,
                                get_luminance_threshold_path(self.cycler),
                                luminance_threshold,
                            );
                        }
                    }
                }
                Err(error) => {
                    ui.label(format!("{error:#}"));
                }
            }
        })
        .response
    }

    fn add_vision_cycler_selector(&mut self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let response = ComboBox::from_label("Cycler")
            .selected_text(format!("{:?}", self.cycler))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut self.cycler, Cycler::VisionTop, "VisionTop")
                    .clicked()
                {
                    changed = true;
                };
                if ui
                    .selectable_value(&mut self.cycler, Cycler::VisionBottom, "VisionBottom")
                    .clicked()
                {
                    changed = true;
                };
            })
            .response;
        if changed {
            self.vertical_edge_threshold = self
                .nao
                .subscribe_parameter(get_vertical_edge_threshold_path(self.cycler));
            self.red_chromaticity_threshold = self
                .nao
                .subscribe_parameter(get_red_chromaticity_threshold_path(self.cycler));
            self.blue_chromaticity_threshold = self
                .nao
                .subscribe_parameter(get_blue_chromaticity_threshold_path(self.cycler));
            self.green_chromaticity_threshold = self
                .nao
                .subscribe_parameter(get_green_chromaticity_threshold_path(self.cycler));
            self.green_luminance_threshold = self
                .nao
                .subscribe_parameter(get_green_luminance_threshold_path(self.cycler));
            self.luminance_threshold = self
                .nao
                .subscribe_parameter(get_luminance_threshold_path(self.cycler));
        }
        response
    }
}

fn get_vertical_edge_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "image_segmenter.vision_top.vertical_edge_threshold",
        Cycler::VisionBottom => "image_segmenter.vision_bottom.vertical_edge_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_red_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.red_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.red_chromaticity_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_blue_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.blue_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.blue_chromaticity_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_green_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.green_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.green_chromaticity_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_green_luminance_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.green_luminance_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.green_luminance_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_luminance_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.luminance_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.luminance_threshold",
        _ => panic!("not implemented"),
    }
}

fn draw_parameter_slider<T>(
    ui: &mut Ui,
    buffer: &ValueBuffer,
    name: impl Into<WidgetText>,
    range: RangeInclusive<T>,
) -> Result<Response>
where
    T: Numeric + Serialize,
    for<'de> T: Deserialize<'de>,
{
    let mut parsed = buffer.parse_latest::<T>()?;

    let response = ui.add(Slider::new(&mut parsed, range).text(name).smart_aim(false));
    if response.changed() {
        buffer.update_parameter_value(to_value(parsed)?);
    }
    Ok(response)
}
