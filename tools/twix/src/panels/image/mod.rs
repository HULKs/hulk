use std::{env::temp_dir, fs::create_dir_all, path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use color_eyre::{
    eyre::{bail, eyre, Context as _},
    Result,
};
use eframe::egui::{
    ColorImage, ComboBox, Context, Response, SizeHint, TextureId, TextureOptions, Ui, Widget,
};
use geometry::rectangle::Rectangle;
use image::{EncodableLayout, RgbImage};
use linear_algebra::{point, vector};
use log::{info, warn};
use ros2::sensor_msgs::image::Image;
use serde_json::{json, Value};

use types::jpeg::JpegImage;

use crate::{
    nao::Nao,
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

use self::overlay::Overlays;

pub mod overlay;
mod overlays;

enum RawOrJpeg {
    Raw(BufferHandle<Image>),
    Jpeg(BufferHandle<JpegImage>),
}

pub struct ImagePanel {
    nao: Arc<Nao>,
    image_buffer: RawOrJpeg,
    overlays: Overlays,
    zoom_and_pan: ZoomAndPanTransform,
    last_image_path: String,
    current_image_path: String,
    current_image_label: String,
}

fn subscribe_image(nao: &Arc<Nao>, is_jpeg: bool, image_path: &str) -> RawOrJpeg {
    if is_jpeg {
        let path = format!("{image_path}.jpeg");
        return RawOrJpeg::Jpeg(nao.subscribe_value(path));
    }
    RawOrJpeg::Raw(nao.subscribe_value(image_path.to_string()))
}

impl<'a> Panel<'a> for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(context: PanelCreationContext) -> Self {
        let is_jpeg = context
            .value
            .and_then(|value| value.get("is_jpeg"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true);

        let default_image_path = "ObjectDetection.main_outputs.image_left_raw".to_string();
        let default_image_label = "Image Left Raw".to_string();

        let image_buffer = subscribe_image(&context.nao, is_jpeg, &default_image_path);

        let overlays = Overlays::new(
            context.nao.clone(),
            context.value.and_then(|value| value.get("overlays")),
        );
        Self {
            nao: context.nao,
            image_buffer,
            overlays,
            zoom_and_pan: ZoomAndPanTransform::default(),
            current_image_path: default_image_path.clone(),
            last_image_path: default_image_path,
            current_image_label: default_image_label,
        }
    }

    fn save(&self) -> Value {
        let overlays = self.overlays.save();

        json!({
            "is_jpeg": matches!(self.image_buffer, RawOrJpeg::Jpeg(_)),
            "cycler": "ObjectDetection",
            "overlays": overlays,
        })
    }
}

fn save_jpeg_image(buffer: &BufferHandle<JpegImage>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_jpeg_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}

fn save_raw_image(buffer: &BufferHandle<Image>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}

impl Widget for &mut ImagePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            let mut jpeg = matches!(self.image_buffer, RawOrJpeg::Jpeg(_));
            self.overlays.combo_box(ui);
            if ui.checkbox(&mut jpeg, "JPEG").changed() {
                self.resubscribe(jpeg);
            }

            ComboBox::from_label("Image Topic")
                .selected_text(self.current_image_label.clone())
                .show_ui(ui, |ui| {
                    let mut selectable_item = |value: &str, label: &str| {
                        let is_selected = self.current_image_path == value;

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.current_image_path = value.to_string();
                            self.current_image_label = label.to_string();
                        }
                    };

                    selectable_item(
                        "ObjectDetection.main_outputs.image_left_raw",
                        "Image Left Raw",
                    );
                    selectable_item("ImageRectified.main_outputs.image", "Rectified Image");
                    selectable_item(
                        "ImageStereonetDepth.main_outputs.image",
                        "StereoNet Depth Image",
                    );
                });
            if self.last_image_path != self.current_image_path {
                self.resubscribe(jpeg);
                self.last_image_path = self.current_image_path.clone();
            }

            let maybe_timestamp = match &self.image_buffer {
                RawOrJpeg::Raw(buffer) => buffer.get_last_timestamp(),
                RawOrJpeg::Jpeg(buffer) => buffer.get_last_timestamp(),
            };
            if let Ok(Some(timestamp)) = maybe_timestamp {
                let date: DateTime<Utc> = timestamp.into();
                ui.label(date.format("%T%.3f").to_string());
            }
            if ui.button("Save").clicked() {
                let time_stamp = Utc::now().format("%H:%M:%S%.3f").to_string();
                let directory = temp_dir().join("twix");
                if let Err(error) = create_dir_all(&directory) {
                    warn!("failed to create temporary folder /tmp/twix: {error}");
                } else {
                    let path = directory.join(format!("image_vision_{time_stamp}.png"));
                    let result = match &self.image_buffer {
                        RawOrJpeg::Raw(buffer) => save_raw_image(buffer, path),
                        RawOrJpeg::Jpeg(buffer) => {
                            save_jpeg_image(buffer, path.with_extension("jpeg"))
                        }
                    };
                    if let Err(error) = result {
                        warn!("failed to save image: {error}");
                    }
                }
            }
        });

        let (texture_id, (width, height)) = match self.load_latest_texture(ui.ctx()) {
            Ok(result) => result,
            Err(error) => {
                return ui.scope(|ui| ui.label(format!("{error}"))).response;
            }
        };

        let (response, mut painter) = TwixPainter::allocate(
            ui,
            vector![width as f32, height as f32],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );
        self.zoom_and_pan.apply(ui, &mut painter, &response);
        painter.image(
            texture_id,
            Rectangle {
                min: point!(0.0, 0.0),
                max: point!(width as f32, height as f32),
            },
        );

        self.overlays.paint(&painter);

        match response.hover_pos() {
            Some(position) => {
                let pixel_position = painter.transform_pixel_to_world(position);
                response.on_hover_text_at_pointer(format!(
                    "x: {:.1}, y: {:.1}",
                    pixel_position.x(),
                    pixel_position.y()
                ))
            }
            _ => response,
        }
    }
}

impl ImagePanel {
    fn resubscribe(&mut self, jpeg: bool) {
        self.image_buffer = subscribe_image(&self.nao, jpeg, &self.current_image_path);
    }

    fn load_latest_texture(&self, context: &Context) -> Result<(TextureId, (u32, u32))> {
        let image_identifier = "bytes://image-vision".to_string();
        match &self.image_buffer {
            RawOrJpeg::Raw(buffer) => {
                let ros_image = buffer
                    .get_last_value()?
                    .ok_or_else(|| eyre!("no image available"))?;
                if ros_image.height == 0 || ros_image.width == 0 {
                    bail!(
                        "Image has no pixels. Dimensions: {}x{}",
                        ros_image.width,
                        ros_image.height
                    );
                }

                let rgb_image: RgbImage = ros_image
                    .try_into()
                    .map_err(|e: image::ImageError| eyre!(e))?;

                let image = ColorImage::from_rgb(
                    [rgb_image.width() as usize, rgb_image.height() as usize],
                    rgb_image.as_bytes(),
                );
                let id = context
                    .load_texture(&image_identifier, image, TextureOptions::NEAREST)
                    .id();

                Ok((id, (rgb_image.width(), rgb_image.height())))
            }
            RawOrJpeg::Jpeg(buffer) => {
                let jpeg = buffer
                    .get_last_value()?
                    .ok_or_else(|| eyre!("no image available"))?;
                let (width, height) = jpeg
                    .dimensions()
                    .wrap_err("failed to read image dimensions")?;
                context.forget_image(&image_identifier);
                context.include_bytes(image_identifier.clone(), jpeg.data);
                let id = context
                    .try_load_texture(
                        &image_identifier,
                        TextureOptions::NEAREST,
                        SizeHint::Size {
                            width,
                            height,
                            maintain_aspect_ratio: true,
                        },
                    )?
                    .texture_id()
                    .unwrap();
                Ok((id, (width, height)))
            }
        }
    }
}
