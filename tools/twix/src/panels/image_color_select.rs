use std::{cmp::Ordering, sync::Arc};

use color_eyre::{eyre::ContextCompat, Result};
use coordinate_systems::Pixel;
use eframe::{
    egui::{
        self, load::SizedTexture, panel::TopBottomSide, CentralPanel, Color32, ColorImage,
        ComboBox, Image, PointerButton, Response, Sense, Stroke, TextureOptions, TopBottomPanel,
        Ui, Widget,
    },
    epaint::Vec2,
};

use egui_plot::{HLine, Points, VLine};
use geometry::rectangle::Rectangle;
use itertools::iproduct;
use linear_algebra::{point, vector, Point2};
use nalgebra::Similarity2;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use types::{
    color::{Rgb, YCbCr444},
    field_color::FieldColor,
    ycbcr422_image::YCbCr422Image,
};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

use super::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

const FIELD_SELECTION_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 0, 0, 50);
const OTHER_SELECTION_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 0, 255, 50);

pub struct ImageColorSelectPanel {
    nao: Arc<Nao>,
    image: BufferHandle<YCbCr422Image>,
    field_color: BufferHandle<FieldColor>,
    cycler: VisionCycler,
    brush_size: f32,
    selection_mask: ColorImage,
    x_axis: Axis,
    y_axis: Axis,
    filter_by_other_axes: bool,
}

impl Panel for ImageColorSelectPanel {
    const NAME: &'static str = "Image Color Select";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let cycler = value
            .and_then(|value| {
                let string = value.get("cycler")?.as_str()?;
                VisionCycler::try_from(string).ok()
            })
            .unwrap_or(VisionCycler::Top);
        let cycler_path = cycler.as_path();
        let image = nao.subscribe_value(format!("{cycler_path}.main_outputs.image"));

        let brush_size = 50.0;

        let selection_mask = ColorImage::new([640, 480], Color32::TRANSPARENT);

        let field_color = nao.subscribe_value(format!("{cycler_path}.main_outputs.field_color"));

        let x_axis = value
            .and_then(|value| serde_json::from_value::<Axis>(value.get("x_axis")?.clone()).ok())
            .unwrap_or(Axis::GreenChromaticity);
        let y_axis = value
            .and_then(|value| serde_json::from_value::<Axis>(value.get("y_axis")?.clone()).ok())
            .unwrap_or(Axis::Luminance);

        let filter_by_other_axes = value
            .and_then(|value| value.get("filter_by_other_axes")?.as_bool())
            .unwrap_or(true);

        Self {
            nao,
            image,
            field_color,
            cycler,
            brush_size,
            selection_mask,
            x_axis,
            y_axis,
            filter_by_other_axes,
        }
    }

    fn save(&self) -> Value {
        json!({
            "cycler": self.cycler.as_path(),
            "x_axis": self.x_axis,
            "y_axis": self.y_axis,
            "filter_by_other_axes": self.filter_by_other_axes,
        })
    }
}

impl Widget for &mut ImageColorSelectPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let image = self.get_image();
        TopBottomPanel::new(TopBottomSide::Bottom, "Franz Josef von Panellington")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("x:");
                    ComboBox::from_id_source("x_axis")
                        .selected_text(format!("{:?}", self.x_axis))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.x_axis,
                                Axis::RedChromaticity,
                                "Red Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.x_axis,
                                Axis::GreenChromaticity,
                                "Green Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.x_axis,
                                Axis::BlueChromaticity,
                                "Blue Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.x_axis,
                                Axis::GreenLuminance,
                                "Green Luminance",
                            );
                            ui.selectable_value(&mut self.x_axis, Axis::Luminance, "Luminance");
                        });
                    ui.label("y:");
                    ComboBox::from_id_source("y_axis")
                        .selected_text(format!("{:?}", self.y_axis))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.y_axis,
                                Axis::RedChromaticity,
                                "Red Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.y_axis,
                                Axis::GreenChromaticity,
                                "Green Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.y_axis,
                                Axis::BlueChromaticity,
                                "Blue Chromaticity",
                            );
                            ui.selectable_value(
                                &mut self.y_axis,
                                Axis::GreenLuminance,
                                "Green Luminance",
                            );
                            ui.selectable_value(&mut self.y_axis, Axis::Luminance, "Luminance");
                        });
                    ui.checkbox(&mut self.filter_by_other_axes, "Filter")
                });

                egui_plot::Plot::new("karsten").show(ui, |plot_ui| {
                    let Ok(Some(field_color)) = self.field_color.get_last_value() else {
                        return;
                    };
                    if let Ok(image) = &image {
                        plot_ui.points(
                            generate_points(
                                image,
                                &self.selection_mask,
                                FIELD_SELECTION_COLOR,
                                self.x_axis,
                                self.y_axis,
                                self.filter_by_other_axes,
                                &field_color,
                            )
                            .color(Color32::RED),
                        );
                        plot_ui.points(
                            generate_points(
                                image,
                                &self.selection_mask,
                                OTHER_SELECTION_COLOR,
                                self.x_axis,
                                self.y_axis,
                                self.filter_by_other_axes,
                                &field_color,
                            )
                            .color(Color32::BLUE),
                        );
                    }
                    plot_ui.vline(VLine::new(self.x_axis.get_threshold(&field_color).1).width(5.0));
                    plot_ui.hline(HLine::new(self.y_axis.get_threshold(&field_color).1).width(5.0));
                })
            });
        CentralPanel::default()
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
                    if cycler_selector.ui(ui).changed() {
                        let cycler_path = self.cycler.as_path();
                        self.image = self
                            .nao
                            .subscribe_value(format!("{cycler_path}.main_outputs.image"));
                        self.field_color = self
                            .nao
                            .subscribe_value(format!("{cycler_path}.main_outputs.field_color"));
                    }

                    if ui.button("reset").clicked() {
                        self.selection_mask = ColorImage::new([640, 480], Color32::TRANSPARENT);
                    };
                    ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=200.0).text("Brush"))
                });
                let image = match image {
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
                let image_widget = Image::new(texture)
                    .shrink_to_fit()
                    .sense(Sense::click_and_drag());
                let mut response = ui.add(image_widget);
                response.rect.set_width(response.rect.width().max(1.0));
                response.rect.set_height(response.rect.height().max(1.0));
                let painter = TwixPainter::<Pixel>::paint_at(ui, response.rect).with_camera(
                    vector![
                        self.selection_mask.width() as f32,
                        self.selection_mask.height() as f32
                    ],
                    Similarity2::identity(),
                    Orientation::LeftHanded,
                );

                if let Some(hover_position) = response.hover_pos() {
                    let pixel_pos = painter.transform_pixel_to_world(hover_position);
                    if pixel_pos.x() < self.selection_mask.width() as f32
                        && pixel_pos.y() < self.selection_mask.height() as f32
                    {
                        let scroll_delta = ui.input(|input| input.raw_scroll_delta);
                        self.brush_size = (self.brush_size + scroll_delta[1]).clamp(1.0, 200.0);
                        if response.is_pointer_button_down_on() {
                            self.pixels_in_brush(pixel_pos, &self.selection_mask)
                                .for_each(|position| {
                                    ui.input(|i| {
                                        if i.pointer.button_down(PointerButton::Primary) {
                                            self.add_to_selection(position, i.modifiers.shift)
                                        }
                                        if i.pointer.button_down(PointerButton::Secondary) {
                                            self.remove_from_selection(position)
                                        }
                                    })
                                });
                        }

                        painter.circle(
                            pixel_pos,
                            self.brush_size,
                            Color32::TRANSPARENT,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                    }
                }

                let colored_handle = ui
                    .ctx()
                    .load_texture(
                        "image",
                        self.selection_mask.clone(),
                        TextureOptions::default(),
                    )
                    .id();
                painter.image(
                    colored_handle,
                    Rectangle {
                        min: point![0.0, 0.0],
                        max: point![640.0, 480.0],
                    },
                );
                response
            })
            .response
    }
}

impl ImageColorSelectPanel {
    fn pixels_in_brush(
        &self,
        brush_position: Point2<Pixel>,
        image: &ColorImage,
    ) -> impl Iterator<Item = (isize, isize)> {
        let brush_size = self.brush_size;
        let width = image.width();
        let height = image.height();
        iproduct!(
            (brush_position.x() as isize - brush_size as isize)
                ..(brush_position.x() as isize + brush_size as isize + 1),
            (brush_position.y() as isize - brush_size as isize)
                ..(brush_position.y() as isize + brush_size as isize + 1)
        )
        .filter(move |(i, j)| {
            ((*i as f32 - brush_position.x()).powi(2) + (*j as f32 - brush_position.y()).powi(2))
                .sqrt()
                <= brush_size
                && (0..width as isize).contains(i)
                && (0..height as isize).contains(j)
        })
    }

    fn add_to_selection(&mut self, position: (isize, isize), other: bool) {
        let color = if other {
            OTHER_SELECTION_COLOR
        } else {
            FIELD_SELECTION_COLOR
        };
        let width = self.selection_mask.width();
        self.selection_mask.pixels[position.1 as usize * width + position.0 as usize] = color;
    }

    fn remove_from_selection(&mut self, position: (isize, isize)) {
        let width = self.selection_mask.width();
        self.selection_mask.pixels[position.1 as usize * width + position.0 as usize] =
            Color32::TRANSPARENT;
    }

    fn get_image(&self) -> Result<ColorImage> {
        let image_ycbcr = self
            .image
            .get_last_value()?
            .wrap_err("No image available")?;

        let buffer = image_ycbcr
            .buffer()
            .iter()
            .flat_map(|&ycbcr422| {
                let ycbcr444: [YCbCr444; 2] = ycbcr422.into();
                ycbcr444
            })
            .flat_map(|ycbcr444| {
                let rgb = Rgb::from(ycbcr444);
                [rgb.red, rgb.green, rgb.blue, 255]
            })
            .collect::<Vec<_>>();
        let image = ColorImage::from_rgba_unmultiplied(
            [image_ycbcr.width() as usize, image_ycbcr.height() as usize],
            &buffer,
        );
        Ok(image)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Axis {
    RedChromaticity,
    GreenChromaticity,
    BlueChromaticity,
    GreenLuminance,
    Luminance,
}

impl Axis {
    fn get_value(self, color: Rgb) -> f32 {
        let chromaticiy = color.convert_to_rgchromaticity();
        match self {
            Axis::RedChromaticity => chromaticiy.red,
            Axis::GreenChromaticity => chromaticiy.green,
            Axis::BlueChromaticity => 1.0 - chromaticiy.red - chromaticiy.green,
            Axis::GreenLuminance => color.green as f32 / 255.0,
            Axis::Luminance => color.get_luminance() as f32 / 255.0,
        }
    }

    fn get_threshold(self, field_color: &FieldColor) -> (Ordering, f32) {
        match self {
            Axis::RedChromaticity => (Ordering::Greater, field_color.red_chromaticity_threshold),
            Axis::GreenChromaticity => (Ordering::Less, field_color.green_chromaticity_threshold),
            Axis::BlueChromaticity => (Ordering::Greater, field_color.blue_chromaticity_threshold),
            Axis::GreenLuminance => (
                Ordering::Less,
                field_color.green_luminance_threshold / 255.0,
            ),
            Axis::Luminance => (Ordering::Greater, field_color.luminance_threshold / 255.0),
        }
    }

    fn passes_threshold(self, color: Rgb, field_color: &FieldColor) -> bool {
        let value = self.get_value(color);
        let (ordering, threshold) = self.get_threshold(field_color);

        value.total_cmp(&threshold) == ordering
    }
}

fn generate_points(
    image: &ColorImage,
    mask: &ColorImage,
    mask_color: Color32,
    x_axis: Axis,
    y_axis: Axis,
    filter_by_other_axes: bool,
    field_color: &FieldColor,
) -> Points {
    Points::new(
        image
            .pixels
            .iter()
            .zip(&mask.pixels)
            .filter_map(|(color, mask)| {
                if *mask != mask_color {
                    return None;
                }
                let rgb = Rgb::new(color.r(), color.g(), color.b());

                    let skip = [x_axis, y_axis];
                    let [
                        red_chromaticity,
                        green_chromaticity,
                        blue_chromaticity,
                        green_luminance,
                        luminance,
                    ] = [
                        Axis::RedChromaticity,
                        Axis::GreenChromaticity,
                        Axis::BlueChromaticity,
                        Axis::GreenLuminance,
                        Axis::Luminance,
                    ]
                    .map(|axis| !skip.contains(&axis) && axis.passes_threshold(rgb, field_color));

                    if filter_by_other_axes
                        && (red_chromaticity
                            || green_chromaticity
                            || blue_chromaticity
                            || green_luminance)
                        && luminance
                    {
                        return None;
                    }

                Some([x_axis.get_value(rgb) as f64, y_axis.get_value(rgb) as f64])
            })
            .collect::<Vec<_>>(),
    )
}
