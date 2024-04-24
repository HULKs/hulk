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
use linear_algebra::{point, vector, Point2};
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
    red: f32,
    green: f32,
    blue: f32,
}

struct ColorArray {
    red: Vec<f64>,
    green: Vec<f64>,
    blue: Vec<f64>,
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
            CoordinateSystem::LeftHand,
        );

        if let Some(hoverpos) = response.hover_pos() {
            let pixel_pos = painter.transform_pixel_to_world(hoverpos);
            if pixel_pos.x() < image.width() as f32 && pixel_pos.y() < image.height() as f32 {
                let scroll_delta = ui.input(|input| input.scroll_delta);
                self.brush_size = (self.brush_size + scroll_delta[1]).clamp(1.0, 200.0);

                let mut max = PixelColor {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                };
                let mut min = PixelColor {
                    red: 1.0,
                    green: 1.0,
                    blue: 1.0,
                };
                let mut average = PixelColor {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                };
                let mut pixel_count: usize = 0;
                let mut color_distribtion = ColorArray {
                    red: vec![0.0; 100],
                    green: vec![0.0; 100],
                    blue: vec![0.0; 100],
                };

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
                            && (0..image.width() as isize).contains(&i)
                            && (0..image.height() as isize).contains(&j)
                        {
                            let circle_pixel = point![i as f32, j as f32];

                            let color = get_pixel_chromaticity(&image, circle_pixel);
                            max.red = max.red.max(color.red);
                            max.green = max.green.max(color.green);
                            max.blue = max.blue.max(color.blue);
                            min.red = min.red.min(color.red);
                            min.green = min.green.min(color.green);
                            min.blue = min.blue.min(color.blue);
                            average.red += color.red;
                            average.green += color.green;
                            average.blue += color.blue;
                            pixel_count += 1;

                            color_distribtion.red[(color.red * 90.0) as usize] += 1.0;
                            color_distribtion.green[(color.green * 90.0) as usize] += 1.0;
                            color_distribtion.blue[(color.blue * 90.0) as usize] += 1.0;
                        }
                    }
                }
                average.red /= pixel_count as f32;
                average.green /= pixel_count as f32;
                average.blue /= pixel_count as f32;

                ui.label(format!(
                    "x: {}\t\ty: {}\t\tpixels: {}\n",
                    pixel_pos.x() as usize,
                    pixel_pos.y() as usize,
                    pixel_count,
                ));

                let grid = egui::Grid::new("colors").num_columns(4).striped(true);
                grid.show(ui, |ui| {
                    ui.label(RichText::new("max:").strong());
                    ui.colored_label(Color32::RED, format!("r: {:.3}", max.red));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", max.green));
                    ui.colored_label(
                        Color32::from_rgb(50, 100, 255),
                        format!("b: {:.3}", max.blue),
                    );
                    ui.end_row();

                    ui.label(RichText::new("min:").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", min.red));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", min.green));
                    ui.colored_label(
                        Color32::from_rgb(50, 100, 255),
                        format!("b: {:.3}", min.blue),
                    );
                    ui.end_row();

                    ui.label(RichText::new("average:").strong());
                    ui.colored_label(Color32::RED, format!(" r: {:.3}", average.red));
                    ui.colored_label(Color32::GREEN, format!("g: {:.3}", average.green));
                    ui.colored_label(
                        Color32::from_rgb(50, 100, 255),
                        format!("b: {:.3}", average.blue),
                    );
                    ui.end_row();
                });

                ui.separator();

                let red_chart = create_chart(color_distribtion.red, Color32::RED, -0.002);
                let green_chart = create_chart(color_distribtion.green, Color32::GREEN, 0.0);
                let blue_chart = create_chart(color_distribtion.blue, Color32::BLUE, 0.002);

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

fn get_pixel_chromaticity(image: &ColorImage, pixel_pos: Point2<Pixel>) -> PixelColor {
    let color32 = image.pixels[(pixel_pos.y() as usize) * image.width() + (pixel_pos.x() as usize)];
    let sum = color32.r() as f32 + color32.g() as f32 + color32.b() as f32;
    let mut pixel = PixelColor {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    if sum != 0.0 {
        pixel.red = (color32.r() as f32) / sum;
        pixel.green = (color32.g() as f32) / sum;
        pixel.blue = (color32.b() as f32) / sum;
    }
    pixel
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
