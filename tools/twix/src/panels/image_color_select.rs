use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::{
    egui::{self, load::SizedTexture, Color32, ColorImage, Response, TextureOptions, Ui, Widget},
    epaint::Vec2,
};

use log::error;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use types::{
    color::{Rgb, YCbCr444},
    ycbcr422_image::YCbCr422Image,
};

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

use super::image::cycler_selector::VisionCyclerSelector;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]

enum ImageKind {
    YCbCr422,
}

pub struct ImageColorSelectPanel {
    nao: Arc<Nao>,
    image_buffer: ValueBuffer,
    cycler_selector: VisionCyclerSelector,
    brush_size: usize,
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
            output: Output::Main {
                path: "image".to_string(),
            },
        };
        let image_buffer = nao.subscribe_output(output);
        let cycler_selector = VisionCyclerSelector::new(cycler);

        let brush_size = 4;
        Self {
            nao,
            image_buffer,
            cycler_selector,
            brush_size,
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
                    output: Output::Main {
                        path: "image".to_string(),
                    },
                };
                self.image_buffer = self.nao.subscribe_output(output);
            }
            ui.add(egui::Slider::new(&mut self.brush_size, 0..=10).text("Brush"));
        });

        let image = match self.get_image() {
            Ok(image) => Some(image),
            Err(error) => {
                ui.label(format!("{error:#?}"));
                None
            }
        };

        let response = if let Some(image) = image {
            let handle = ui
                .ctx()
                .load_texture("image", image.clone(), TextureOptions::default())
                .id();
            let texture = SizedTexture {
                id: handle,
                size: Vec2::new(640.0, 480.0),
            };
            let response = ui.image(texture);
            if let Some(hoverpos) = response.hover_pos() {
                let min = response.rect.min;
                let max = response.rect.max;
                let pixel_pos = (hoverpos - min) * Vec2::new(640.0, 480.0) / (max - min);

                if pixel_pos[0] <= 640.0 && pixel_pos[1] <= 480.0 {
                    let color = get_pixel_color(image, pixel_pos);

                    let r = (color.r() as f32) / ((color.r() as f32 + color.g() as f32 + color.b() as f32));
                    let g = (color.g() as f32) / ((color.r() as f32 + color.g() as f32 + color.b() as f32));
                    let b = (color.b() as f32) / ((color.r() as f32 + color.g() as f32 + color.b() as f32));

                    ui.label(format!(
                        "x: {}, y: {} \nr: {:.3}, g: {:.3}, b: {:.3}",
                        pixel_pos[0] as usize, pixel_pos[1] as usize, r, g, b
                    ));
                }
            }
            response
        } else {
            ui.label("Error: Could not display image")
        };
        response
    }
}

impl ImageColorSelectPanel {
    fn get_image(&self) -> Result<ColorImage> {
        let image_data: YCbCr422Image = self
            .image_buffer
            .parse_latest()
            .map_err(|error| eyre!("{error}"))?;
        let buffer = image_data
            .buffer()
            .iter()
            .flat_map(|&ycbcr422| {
                let ycbcr444: [YCbCr444; 2] = ycbcr422.into();
                ycbcr444
            })
            .flat_map(|ycbcr444| {
                let rgb = Rgb::from(ycbcr444);
                [rgb.r, rgb.g, rgb.b, 255]
            })
            .collect::<Vec<_>>();
        let image = ColorImage::from_rgba_unmultiplied([640, 480], &buffer);
        Ok(image)
    }
}

fn get_pixel_color(image: ColorImage, pixel_pos: Vec2) -> Color32 {
    image.pixels[(pixel_pos[1] as usize) * 640 + (pixel_pos[0] as usize)]
}
