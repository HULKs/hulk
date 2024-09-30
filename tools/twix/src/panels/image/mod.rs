use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use eframe::{
    egui::{ComboBox, Image, Response, TextureOptions, Ui, Widget},
    epaint::Vec2,
};
use log::error;
use nalgebra::Similarity2;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use communication::client::{Cycler, CyclerOutput, Output};
use linear_algebra::vector;

use crate::{
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    twix_painter::{CoordinateSystem, TwixPainter},
};

use self::{cycler_selector::VisionCyclerSelector, overlay::Overlays};

mod cycler_selector;
mod overlay;
mod overlays;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
enum ImageKind {
    YCbCr422,
    Luminance,
}

impl ImageKind {
    fn as_output(&self) -> Output {
        match self {
            ImageKind::YCbCr422 => Output::Main {
                path: "image.jpeg".to_string(),
            },
            ImageKind::Luminance => Output::Additional {
                path: "robot_detection.luminance_image.jpeg".to_string(),
            },
        }
    }
}

pub struct ImagePanel {
    nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    cycler_selector: VisionCyclerSelector,
    overlays: Overlays,
    image_kind: ImageKind,
}

impl Panel for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let cycler = value
            .and_then(|value| value.get("cycler"))
            .and_then(|value| value.as_str())
            .map(Cycler::from_str)
            .and_then(|cycler| match cycler {
                Ok(cycler @ (Cycler::VisionTop | Cycler::VisionBottom)) => Some(cycler),
                Ok(cycler) => {
                    error!("Invalid vision cycler: {cycler}");
                    None
                }
                Err(error) => {
                    error!("{error}");
                    None
                }
            })
            .unwrap_or(Cycler::VisionTop);
        let image_kind = value
            .and_then(|value| value.get("image_kind"))
            .and_then(|value| from_value(value.clone()).ok())
            .unwrap_or(ImageKind::YCbCr422);
        let output = CyclerOutput {
            cycler,
            output: image_kind.as_output(),
        };
        let image_buffer = nao.subscribe_image(output);
        let cycler_selector = VisionCyclerSelector::new(cycler);
        let overlays = Overlays::new(
            nao.clone(),
            value.and_then(|value| value.get("overlays")),
            cycler_selector.selected_cycler(),
        );
        Self {
            nao,
            image_buffer,
            cycler_selector,
            overlays,
            image_kind,
        }
    }

    fn save(&self) -> Value {
        let cycler = self.cycler_selector.selected_cycler();
        let overlays = self.overlays.save();
        let image_kind = format!("{:?}", self.image_kind);

        json!({
            "cycler": cycler.to_string(),
            "overlays": overlays,
            "image_kind": image_kind,
        })
    }
}

impl Widget for &mut ImagePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if self.cycler_selector.ui(ui).changed() {
                let output = CyclerOutput {
                    cycler: self.cycler_selector.selected_cycler(),
                    output: self.image_kind.as_output(),
                };
                self.image_buffer = self.nao.subscribe_image(output);
                self.overlays
                    .update_cycler(self.cycler_selector.selected_cycler());
            }
            let mut image_selection_changed = false;
            ComboBox::from_label("Image")
                .selected_text(format!("{:?}", self.image_kind))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut self.image_kind, ImageKind::YCbCr422, "YCbCr422")
                        .changed()
                    {
                        image_selection_changed = true;
                    };
                    if ui
                        .selectable_value(&mut self.image_kind, ImageKind::Luminance, "Luminance")
                        .changed()
                    {
                        image_selection_changed = true;
                    }
                });
            if image_selection_changed {
                let output = CyclerOutput {
                    cycler: self.cycler_selector.selected_cycler(),
                    output: self.image_kind.as_output(),
                };
                self.image_buffer = self.nao.subscribe_image(output);
                self.overlays
                    .update_cycler(self.cycler_selector.selected_cycler());
            }
            self.overlays
                .combo_box(ui, self.cycler_selector.selected_cycler());
        });

        match self.show_image(ui) {
            Ok(response) => response,
            Err(error) => ui.label(format!("{error:#?}")),
        }
    }
}

impl ImagePanel {
    fn show_image(&self, ui: &mut Ui) -> Result<Response> {
        let image_data = self
            .image_buffer
            .get_latest()
            .map_err(|error| eyre!("{error}"))?;
        let image_raw = bincode::deserialize::<Vec<u8>>(&image_data)?;

        let image_identifier = format!("bytes://image-{:?}", self.cycler_selector);
        ui.ctx().forget_image(&image_identifier);
        let image = Image::from_bytes(image_identifier, image_raw)
            .texture_options(TextureOptions::NEAREST)
            .fit_to_fraction(Vec2::splat(1.0));

        let image_response = ui.add(image);

        let painter = TwixPainter::paint_at(ui, image_response.rect).with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );
        let _ = self.overlays.paint(&painter);
        Ok(image_response)
    }
}
