use std::sync::Arc;

use anyhow::{anyhow, Result};
use communication::Cycler;
use eframe::{
    egui::{Response, Ui, Widget},
    emath::Rect,
    Storage,
};
use egui_extras::RetainedImage;
use log::error;
use nalgebra::{vector, Similarity2};

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

pub struct ImagePanel {
    nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    cycler_selector: VisionCyclerSelector,
    overlays: Overlays,
}

impl Panel for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>) -> Self {
        let cycler_name = storage
            .and_then(|storage| storage.get_string("image.cycler"))
            .unwrap_or_else(|| "vision_top".to_string());
        let cycler = match cycler_name.as_str() {
            "vision_top" => Cycler::VisionTop,
            "vision_bottom" => Cycler::VisionBottom,
            _ => panic!("Unknown cycler '{cycler_name}'"),
        };
        let image_buffer = nao.subscribe_image(cycler);
        let cycler_selector = VisionCyclerSelector::new(cycler);
        let overlays = Overlays::new(nao.clone(), storage, cycler_selector.selected_cycler());
        Self {
            nao,
            image_buffer,
            cycler_selector,
            overlays,
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        match self.cycler_selector.selected_cycler() {
            Cycler::VisionTop => storage.set_string("image.cycler", "vision_top".to_string()),
            Cycler::VisionBottom => storage.set_string("image.cycler", "vision_bottom".to_string()),
            cycler => error!("Invalid camera cycler: {cycler}"),
        }

        self.overlays.save(storage)
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
            .map_err(|error| anyhow!("{error}"))?;
        let image = RetainedImage::from_image_bytes("image", &image_data)
            .map_err(|error| anyhow!("{error}"))?;
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
