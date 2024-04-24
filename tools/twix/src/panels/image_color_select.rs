use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::{
    egui::{
        self, load::SizedTexture, Color32, ColorImage, Response, RichText, Stroke, TextureOptions,
        Ui, Widget,
    },
    epaint::Vec2,
};

use egui_plot::{Bar, BarChart};
use itertools::iproduct;
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
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
};

use super::image::cycler_selector::VisionCyclerSelector;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]

enum ImageKind {
    YCbCr422,
}

struct PixelColor {
    red: f32,
    green: f32,
    blue: f32,
}

impl PixelColor {
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    pub const WHITE: Self = Self {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
    };
}

struct ColorArray {
    red: Vec<f64>,
    green: Vec<f64>,
    blue: Vec<f64>,
}
impl Default for ColorArray {
    fn default() -> Self {
        Self {
            red: vec![0.0; 50],
            green: vec![0.0; 50],
            blue: vec![0.0; 50],
        }
    }
}
struct Statistics {
    max: PixelColor,
    min: PixelColor,
    average: PixelColor,
    color_distribution: ColorArray,
    pixel_count: usize,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            max: PixelColor::BLACK,
            min: PixelColor::WHITE,
            average: PixelColor::BLACK,
            color_distribution: ColorArray::default(),
            pixel_count: 0,
        }
    }
}

impl Statistics {
    fn sample(mut self, pixel: Color32) -> Self {
        let sum = pixel.r() as f32 + pixel.g() as f32 + pixel.b() as f32;
        let mut pixel_color = PixelColor {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
        };
        if sum != 0.0 {
            pixel_color.red = (pixel.r() as f32) / sum;
            pixel_color.green = (pixel.g() as f32) / sum;
            pixel_color.blue = (pixel.b() as f32) / sum;
        }

        self.max.red = self.max.red.max(pixel_color.red);
        self.max.green = self.max.green.max(pixel_color.green);
        self.max.blue = self.max.blue.max(pixel_color.blue);
        self.min.red = self.min.red.min(pixel_color.red);
        self.min.green = self.min.green.min(pixel_color.green);
        self.min.blue = self.min.blue.min(pixel_color.blue);
        self.average.red += pixel_color.red;
        self.average.green += pixel_color.green;
        self.average.blue += pixel_color.blue;
        self.pixel_count += 1;
        self
    }
}

pub struct ImageColorSelectPanel {
    nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    cycler_selector: VisionCyclerSelector,
    brush_size: f32,
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
        let image_buffer = nao.subscribe_image(output);
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
                self.image_buffer = self.nao.subscribe_image(output);
            }
            ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=200.0).text("Brush"));
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
            size: Vec2::new(image.width() as f32, image.height() as f32),
        };
        let response = ui.image(texture);

        ui.separator();

        let painter = TwixPainter::<Pixel>::paint_at(ui, response.rect).with_camera(
            vector![image.width() as f32, image.height() as f32],
            Similarity2::identity(),
            Orientation::LeftHanded,
        );

        if let Some(hover_position) = response.hover_pos() {
            let pixel_pos = painter.transform_pixel_to_world(hover_position);
            if pixel_pos.x() < image.width() as f32 && pixel_pos.y() < image.height() as f32 {
                let scroll_delta = ui.input(|input| input.raw_scroll_delta);
                self.brush_size = (self.brush_size + scroll_delta[1]).clamp(1.0, 200.0);
                let mut statistics = self
                    .pixels_in_brush(pixel_pos, &image)
                    .fold(Statistics::default(), Statistics::sample);

                if statistics.pixel_count != 0 {
                    statistics.average.red /= statistics.pixel_count as f32;
                    statistics.average.green /= statistics.pixel_count as f32;
                    statistics.average.blue /= statistics.pixel_count as f32;
                }
                ui.label(format!(
                    "x: {}\t\ty: {}\t\tpixels: {}\n",
                    pixel_pos.x() as usize,
                    pixel_pos.y() as usize,
                    statistics.pixel_count,
                ));

                let grid = egui::Grid::new("colors").num_columns(4).striped(true);
                grid.show(ui, |ui| {
                    ui.label(RichText::new("max:").strong());
                    ui.colored_label(Color32::RED, format!("r: {:.3}", statistics.max.red));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", statistics.max.green));
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}", statistics.max.blue),
                    );
                    ui.end_row();

                    ui.label(RichText::new("min:").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", statistics.min.red));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", statistics.min.green));
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}", statistics.min.blue),
                    );
                    ui.end_row();

                    ui.label(RichText::new("average:").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", statistics.average.red));
                    ui.colored_label(
                        Color32::GREEN,
                        format!("g: {:.3}", statistics.average.green),
                    );
                    ui.colored_label(
                        Color32::from_rgb(50, 150, 255),
                        format!("b: {:.3}", statistics.average.blue),
                    );
                    ui.end_row();
                });

                ui.separator();

                let red_chart = create_chart(statistics.color_distribution.red, Color32::RED, -0.002);
                let green_chart = create_chart(statistics.color_distribution.green, Color32::GREEN, 0.0);
                let blue_chart = create_chart(statistics.color_distribution.blue, Color32::BLUE, 0.002);

                egui_plot::Plot::new("karsten").show(ui, |plot_ui| {
                    plot_ui.bar_chart(red_chart);
                    plot_ui.bar_chart(green_chart);
                    plot_ui.bar_chart(blue_chart);
                });

                painter.circle(
                    pixel_pos,
                    self.brush_size,
                    Color32::TRANSPARENT,
                    Stroke::new(1.0, Color32::BLACK),
                );
            }
        }
        response
    }
}

impl<'a> ImageColorSelectPanel {
    fn pixels_in_brush(
        &'a self,
        brush_position: Point2<Pixel>,
        image: &'a ColorImage,
    ) -> impl Iterator<Item = Color32> + 'a {
        iproduct!(
            (brush_position.x() as isize - self.brush_size as isize)
                ..(brush_position.x() as isize + self.brush_size as isize + 1),
            (brush_position.y() as isize - self.brush_size as isize)
                ..(brush_position.y() as isize + self.brush_size as isize + 1)
        )
        .filter(move |(i, j)| {
            ((*i as f32 - brush_position.x()).powi(2) + (*j as f32 - brush_position.y()).powi(2))
                .sqrt()
                <= self.brush_size
                && (0..image.width() as isize).contains(i)
                && (0..image.height() as isize).contains(j)
        })
        .map(|(i, j)| image.pixels[j as usize * image.width() + i as usize])
    }

    fn get_image(&self) -> Result<ColorImage> {
        let buffer = self
            .image_buffer
            .get_latest()
            .map_err(|error| eyre!("{error}"))?;
        let image_ycbcr = bincode::deserialize::<YCbCr422Image>(&buffer)?;

        let rgb_bytes = image_ycbcr
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
        let image = ColorImage::from_rgba_unmultiplied(
            [image_ycbcr.width() as usize, image_ycbcr.height() as usize],
            &rgb_bytes,
        );
        Ok(image)
    }
}

fn create_chart(vector: Vec<f64>, color: Color32, offset: f64) -> BarChart {
    BarChart::new(
        vector
            .iter()
            .enumerate()
            .map(|(index, &value)| Bar::new(index as f64 * 0.01 + offset, value))
            .collect(),
    )
    .color(color)
    .width(0.002)
}
