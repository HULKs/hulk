use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::{
    egui::{
        self, load::SizedTexture, Color32, ColorImage, Pos2, Response, RichText, Stroke,
        TextureOptions, Ui, Widget,
    },
    epaint::Vec2,
};

use linear_algebra::{vector, Point2};
use log::error;

use nalgebra::Similarity2;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use types::{
    color::{Rgb, YCbCr444},
    ycbcr422_image::YCbCr422Image,
};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{CoordinateSystem, TwixPainter},
    value_buffer::ValueBuffer,
};

use super::image::cycler_selector::VisionCyclerSelector;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]

enum ImageKind {
    YCbCr422,
}

pub struct ImageColorSelectPanel {
    nao: Arc<Nao>,
    image_buffer: ValueBuffer,
    cycler_selector: VisionCyclerSelector,
    brush_size: f32,
}
struct PixelColor {
    r: f32,
    g: f32,
    b: f32,
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

        let brush_size = 50.0;
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
            ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=200.0).text("Brush"));
            let scroll_delta = ui.input(|input| input.scroll_delta);
            self.brush_size = (self.brush_size + scroll_delta[1]).clamp(1.0, 200.0);
        });

        ui.separator();

        let image = match self.get_image() {
            Ok(image) => image,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };

        let handle = ui
            .ctx()
            .load_texture("image", image.clone(), TextureOptions::default())
            .id();
        let texture = SizedTexture {
            id: handle,
            size: Vec2::new(640.0, 480.0),
        };
        let response = ui.image(texture);

        ui.separator();

        let painter = TwixPainter::<Pixel>::paint_at(ui, response.rect).with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );

        if let Some(hoverpos) = response.hover_pos() {
            let pixel_pos = painter.transform_pixel_to_world(hoverpos);
            if pixel_pos.x() < 640.0 && pixel_pos.y() < 480.0 {
                let mut max = PixelColor {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                };
                let mut min = PixelColor {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                };
                let mut average = PixelColor {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                };
                let mut cnt: usize = 0;

                for i in (pixel_pos.x() as isize - self.brush_size as isize)
                    ..(pixel_pos.x() as isize + self.brush_size as isize + 1)
                {
                    for j in (pixel_pos.y() as isize - self.brush_size as isize)
                        ..(pixel_pos.y() as isize + self.brush_size as isize + 1)
                    {
                        if f32::sqrt(
                            f32::powi(i as f32 - pixel_pos.x(), 2)
                                + f32::powi(j as f32 - pixel_pos.y(), 2),
                        ) <= self.brush_size
                        {
                            if i < 640 && i > 0 && j < 480 && j > 0 {
                                let circle_pixel = painter.transform_pixel_to_world(Pos2 {
                                    x: i as f32,
                                    y: j as f32,
                                });
                                let color = get_pixel_color(&image, circle_pixel);
                                max.r = max.r.max(color.r);
                                max.g = max.g.max(color.g);
                                max.b = max.b.max(color.b);
                                min.r = min.r.min(color.r);
                                min.g = min.g.min(color.g);
                                min.b = min.b.min(color.b);
                                average.r += color.r;
                                average.g += color.g;
                                average.b += color.b;
                                cnt += 1;
                            }
                        }
                    }
                }
                average.r = average.r / cnt as f32;
                average.g = average.g / cnt as f32;
                average.b = average.b / cnt as f32;

                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "   x: {}\t\ty: {}\t  pixels: {}\n",
                        pixel_pos.x() as usize,
                        pixel_pos.y() as usize,
                        cnt,
                    ));
                    ui.label(RichText::new("max:\t\t").strong());
                    ui.colored_label(Color32::RED, format!("r: {:.3}", max.r));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", max.g));
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}\n", max.b),
                    );

                    ui.label(RichText::new("min:\t\t").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", min.r));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", min.g));
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}\n", min.b),
                    );

                    ui.label(RichText::new("average:").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", average.r));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", average.g));
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}", average.b),
                    );

                    painter.circle(
                        pixel_pos,
                        self.brush_size as f32,
                        Color32::TRANSPARENT,
                        Stroke::new(1.0, Color32::BLACK),
                    );
                });
            }
        }
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

fn get_pixel_color(image: &ColorImage, pixel_pos: Point2<Pixel>) -> PixelColor {
    let color32 = image.pixels[(pixel_pos.y() as usize) * 640 + (pixel_pos.x() as usize)];
    let sum = color32.r() as f32 + color32.g() as f32 + color32.b() as f32;
    PixelColor {
        r: (color32.r() as f32) / sum,
        g: (color32.g() as f32) / sum,
        b: (color32.b() as f32) / sum,
    }
}
