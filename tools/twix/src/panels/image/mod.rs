use std::{str::FromStr, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::client::Cycler;
use eframe::{
    egui::{Response, Ui, Widget},
    emath::Rect,
};
use egui_extras::RetainedImage;
use log::error;
use nalgebra::{vector, Similarity2};
use serde_json::{json, Value};

use crate::{
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    raw_image::RawImage,
    twix_painter::{CoordinateSystem, TwixPainter},
};

use self::{cycler_selector::VisionCyclerSelector, overlay::Overlays};

mod cycler_selector;
mod overlay;
mod overlays;

pub struct ImagePanel {
    nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    cycler_selector: VisionCyclerSelector,
    overlays: Overlays,
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
        let image_buffer = nao.subscribe_image(cycler);
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
                self.image_buffer = self
                    .nao
                    .subscribe_image(self.cycler_selector.selected_cycler());
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
        let image_raw = bincode::deserialize::<RawImage>(&image_data)?;
        let image = RetainedImage::from_image_bytes("image", &image_raw.buffer)
            .map_err(|error| eyre!("{error}"))?;
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
