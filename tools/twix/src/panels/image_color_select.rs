use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::{
    egui::{load::SizedTexture, Color32, ColorImage, Image, ImageFit, Response, Sense, TextureOptions, Ui, Widget},
    epaint::Vec2,
};

use log::error;

use linear_algebra::vector;
use nalgebra::Similarity2;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use types::{color::{Rgb, YCbCr422, YCbCr444}, image_segments::ImageSegments, ycbcr422_image::{self, YCbCr422Image}};

use crate::{
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    twix_painter::{CoordinateSystem, TwixPainter}, value_buffer::ValueBuffer,
};

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
    image_buffer: ValueBuffer,
    cycler_selector: VisionCyclerSelector,
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
        let output = CyclerOutput {
            cycler,
            output: Output::Main{path:"image".to_string()},
        };
        let image_buffer = nao.subscribe_output(output);
        let cycler_selector = VisionCyclerSelector::new(cycler);

        Self {
            nao,
            image_buffer,
            cycler_selector,
        }
    }

    fn save(&self) -> Value {
        let cycler = self.cycler_selector.selected_cycler();
        json!({
            "cycler": cycler.to_string(),
        })
    }
}

impl Widget for &mut ImageColorSelectPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if self.cycler_selector.ui(ui).changed() {
                let output = CyclerOutput {
                    cycler: self.cycler_selector.selected_cycler(),
                    output: Output::Main{path:"image".to_string()},
                };
                self.image_buffer = self.nao.subscribe_output(output);
            }
        });
        let image = match self.get_image() {
            Ok(image) => Some(image),
            Err(error) => {
                ui.label(format!("{error:#?}"));
                None
            }
        };
        let response = if let Some(image) = image { 
            let handle = ui.ctx().load_texture("image", image, TextureOptions::default()).id();
            let texture = SizedTexture{
                id: handle,
                size: Vec2::new(640.0, 480.0),
            };
            let response = ui.image(texture);
            if let Some(hoverpos) = response.hover_pos(){
                let min = response.rect.min;
                let max = response.rect.max;
                let pixel_pos = (hoverpos-min)/(max-min)*Vec2::new(640.0, 480.0).round();
                //image.pixels
                //ui.label(format!("x: {}, y: {}"))
            }
            
            response  
        }
        else{
            ui.label("Error: Could not display image")
        };
        response
        }

}

impl ImageColorSelectPanel {
    fn get_image(&self) -> Result<ColorImage>{
        let image_data:YCbCr422Image = self
            .image_buffer
            .parse_latest()
            .map_err(|error| eyre!("{error}"))?;
        let buffer = image_data.buffer().iter().flat_map(|&ycbcr422|{
            let ycbcr444:[YCbCr444;2] = ycbcr422.into();
            ycbcr444
        }).flat_map(|ycbcr444|{
            let rgb = Rgb::from(ycbcr444);
            [rgb.r, rgb.g, rgb.b, 255]
        }).collect::<Vec<_>>();
        let image = ColorImage::from_rgba_unmultiplied([640,480], &buffer);
        Ok(image)
    }
}


