use std::{env::temp_dir, fs::create_dir_all, path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use color_eyre::{
    eyre::{bail, eyre, Context as _},
    Result,
};
use eframe::egui::{
    ColorImage, Context, Response, SizeHint, TextureId, TextureOptions, Ui, Widget,
};
use geometry::rectangle::Rectangle;
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
    show_depth_image: bool,
    image_buffer: RawOrJpeg,
    overlays: Overlays,
    zoom_and_pan: ZoomAndPanTransform,
}

fn subscribe_image(nao: &Arc<Nao>, is_jpeg: bool, is_depth: bool) -> RawOrJpeg {
    let base_name = if is_depth { "depth_image" } else { "image" };
    if is_jpeg {
        let path = format!("ObjectDetection.main_outputs.{base_name}.jpeg");
        return RawOrJpeg::Jpeg(nao.subscribe_value(path));
    }
    let path = format!("ObjectDetection.main_outputs.{base_name}");
    RawOrJpeg::Raw(nao.subscribe_value(path))
}

impl<'a> Panel<'a> for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(context: PanelCreationContext) -> Self {
        let is_jpeg = context
            .value
            .and_then(|value| value.get("is_jpeg"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true);

        let image_buffer = subscribe_image(&context.nao, is_jpeg, false);

        let overlays = Overlays::new(
            context.nao.clone(),
            context.value.and_then(|value| value.get("overlays")),
        );
        Self {
            nao: context.nao,
            image_buffer,
            overlays,
            zoom_and_pan: ZoomAndPanTransform::default(),
            show_depth_image: false,
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
            if ui.checkbox(&mut self.show_depth_image, "Depth").changed() {
                self.resubscribe(jpeg);
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
        self.image_buffer = subscribe_image(&self.nao, jpeg, self.show_depth_image);
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
                let image = ColorImage::from_rgb(
                    [ros_image.width as usize, ros_image.height as usize],
                    &ros_image.data,
                );
                let id = context
                    .load_texture(&image_identifier, image, TextureOptions::NEAREST)
                    .id();

                Ok((id, (ros_image.width, ros_image.height)))
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
