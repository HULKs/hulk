use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::Cycler;
use eframe::{
    egui::{ComboBox, Response, TextureFilter, Ui, Widget},
    emath::Rect,
};
use egui_extras::RetainedImage;
use log::error;
use nalgebra::{vector, Similarity2};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

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
    fn as_path(&self) -> &str {
        match self {
            ImageKind::YCbCr422 => "image",
            ImageKind::Luminance => "luminance_image",
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
        let image_buffer = nao.subscribe_image(cycler, image_kind.as_path());
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

        json!({
            "cycler": cycler.to_string(),
            "overlays":overlays,
        })
    }
}

impl Widget for &mut ImagePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if self.cycler_selector.ui(ui).changed() {
                self.image_buffer = self.nao.subscribe_image(
                    self.cycler_selector.selected_cycler(),
                    self.image_kind.as_path(),
                );
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
                self.image_buffer = self.nao.subscribe_image(
                    self.cycler_selector.selected_cycler(),
                    self.image_kind.as_path(),
                );
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
        let image = RetainedImage::from_image_bytes("image", &image_raw)
            .map_err(|error| eyre!("{error}"))?
            .with_texture_filter(TextureFilter::Nearest);
        let image_size = image.size_vec2();
        let width_scale = ui.available_width() / image_size.x;
        let height_scale = ui.available_height() / image_size.y;
        let scale = width_scale.min(height_scale);
        let image_response = image.show_scaled(ui, scale);
        let displayed_image_size = image_size * scale;
        let image_rect = Rect::from_min_size(image_response.rect.left_top(), displayed_image_size);
        let painter = TwixPainter::paint_at(ui, image_rect).with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );
        let _ = self.overlays.paint(&painter);
        Ok(image_response)
    }
}
