use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::{
    egui::{Image, Response, TextureOptions, Ui, Widget},
    epaint::Vec2,
};

use log::error;

use nalgebra::Similarity2;
use linear_algebra::vector;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use types::image_segments::ImageSegments;

use crate::{image_buffer::ImageBuffer, nao::Nao, panel::Panel, twix_painter::{TwixPainter, CoordinateSystem}};

use super::image::cycler_selector::VisionCyclerSelector;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]

enum ImageKind {
    YCbCr422,
}

impl ImageKind {
    fn as_output(&self) -> Output {
        Output::Main {
            path: "image.jpeg".to_string(),
        }
    }
}

pub struct ImageColorSelectPanel {
    nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    cycler_selector: VisionCyclerSelector,
    image_kind: ImageKind,
}

impl Panel for ImageColorSelectPanel {
    const NAME: &'static str = "Image Color Select";

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

        Self {
            nao,
            image_buffer,
            cycler_selector,

            image_kind,
        }
    }

    fn save(&self) -> Value {
        let cycler = self.cycler_selector.selected_cycler();
        let image_kind = format!("{:?}", self.image_kind);

        json!({
            "cycler": cycler.to_string(),
            "image_kind": image_kind,
        })
    }
}

impl Widget for &mut ImageColorSelectPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if self.cycler_selector.ui(ui).changed() {
                let output = CyclerOutput {
                    cycler: self.cycler_selector.selected_cycler(),
                    output: self.image_kind.as_output(),
                };
                self.image_buffer = self.nao.subscribe_image(output);

            }
            let (mut response, painter) = TwixPainter::<Pixel>::allocate_new(ui);
        let painter = painter.with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );
        if let Some(hover_pos) = response.hover_pos() {
            let image_coords = painter.transform_pixel_to_world(hover_pos);
            let x = image_coords.x().round() as u16;
            let y = image_coords.y().round() as u16;
            
            if let Some(hover_pos) = response.hover_pos() {
                let image_coords = painter.transform_pixel_to_world(hover_pos);
                let x = image_coords.x().round() as u16;
                let y = image_coords.y().round() as u16;
            
                // Get the pixel color at the hover position
                if let Some(pixel) = self.image_buffer.get_pixel(x, y) {
                    let rgb = pixel.to_rgb();
            
                    // Display the RGB color in the label
                    ui.label(format!("x: {}, y: {}, RGB: ({}, {}, {})", x, y, rgb.r, rgb.g, rgb.b));
                }
            };
        };
        
        //-----------------------------------------

        });

        match self.show_image(ui) {
            Ok(response) => response,
            Err(error) => ui.label(format!("{error:#?}")),
        }
    }
}

impl ImageColorSelectPanel {
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

        Ok(image_response)
    }
}
