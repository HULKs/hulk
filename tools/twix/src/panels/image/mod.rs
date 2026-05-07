use std::{env::temp_dir, fs::create_dir_all, path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use color_eyre::{
    Result,
    eyre::{Context as _, bail, eyre},
};
use eframe::egui::{
    ColorImage, ComboBox, Context, Response, SizeHint, TextureId, TextureOptions, Ui, Widget,
};
use geometry::rectangle::Rectangle;
use image::{EncodableLayout, RgbImage};
use linear_algebra::{point, vector};
use log::{info, warn};
use ros2::sensor_msgs::image::Image as Ros2Image;
use serde_json::{Value, json};

use types::{jpeg::JpegImage, ycbcr422_image::YCbCr422Image};

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    topic_completion_edit::TopicCompletionEdit,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

use self::overlay::Overlays;

pub mod overlay;
mod overlays;

enum ImageBuffer {
    LegacyRaw(BufferHandle<Ros2Image>),
    YCbCr(BufferHandle<YCbCr422Image>),
    Jpeg(BufferHandle<JpegImage>),
    UnsupportedTopic,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageSourceMode {
    Topic,
    Legacy,
}

pub struct ImagePanel {
    robot: Arc<Robot>,
    image_buffer: ImageBuffer,
    overlays: Overlays,
    zoom_and_pan: ZoomAndPanTransform,
    source_mode: ImageSourceMode,
    use_jpeg: bool,
    current_image_path: String,
    current_topic: String,
}

const DEFAULT_IMAGE_PATH: &str = "Vision.main_outputs.image";
const DEFAULT_TOPIC: &str = "/alpha/robot_hw/left_image";
const LEGACY_YCBCR_PATH: &str = "Vision.main_outputs.ycbcr422_image";

fn subscribe_image(
    robot: &Arc<Robot>,
    source_mode: ImageSourceMode,
    is_jpeg: bool,
    image_path: &str,
    _topic: &str,
) -> ImageBuffer {
    if source_mode == ImageSourceMode::Topic {
        return ImageBuffer::UnsupportedTopic;
    }

    if is_jpeg {
        let path = format!("{image_path}.jpeg");
        ImageBuffer::Jpeg(robot.subscribe_value(path))
    } else if image_path.ends_with("ycbcr422_image") {
        ImageBuffer::YCbCr(robot.subscribe_value(image_path.to_string()))
    } else {
        ImageBuffer::LegacyRaw(robot.subscribe_value(image_path.to_string()))
    }
}

impl ImageSourceMode {
    fn from_value(value: Option<&Value>) -> Self {
        match value
            .and_then(|value| value.get("source"))
            .and_then(Value::as_str)
        {
            Some("legacy") => Self::Legacy,
            Some("topic") => Self::Topic,
            _ => Self::Legacy,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Topic => "topic",
            Self::Legacy => "legacy",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Topic => "ROS-Z Topic",
            Self::Legacy => "Legacy Path",
        }
    }
}

fn topic_image_unavailable_message() -> &'static str {
    "ROS-Z topic images are unavailable until typed topic subscriptions are restored. Use Legacy Path for images or Text/Plot/Enum panels for dynamic ROS-Z topics."
}

fn legacy_image_label(image_path: &str) -> &str {
    match image_path {
        DEFAULT_IMAGE_PATH => "Image Left Raw",
        LEGACY_YCBCR_PATH => "ycbcr422_image",
        _ => image_path,
    }
}

impl<'a> Panel<'a> for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(context: PanelCreationContext) -> Self {
        let use_jpeg = context
            .value
            .and_then(|value| value.get("is_jpeg"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let source_mode = ImageSourceMode::from_value(context.value);
        let current_image_path = context
            .value
            .and_then(|value| value.get("image_path"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_IMAGE_PATH)
            .to_string();
        let current_topic = context
            .value
            .and_then(|value| value.get("topic"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_TOPIC)
            .to_string();

        let image_buffer = subscribe_image(
            &context.robot,
            source_mode,
            use_jpeg,
            &current_image_path,
            &current_topic,
        );

        let overlays = Overlays::new(
            context.robot.clone(),
            context.value.and_then(|value| value.get("overlays")),
        );
        Self {
            robot: context.robot,
            image_buffer,
            overlays,
            zoom_and_pan: ZoomAndPanTransform::default(),
            source_mode,
            use_jpeg,
            current_image_path,
            current_topic,
        }
    }

    fn save(&self) -> Value {
        let overlays = self.overlays.save();

        json!({
            "is_jpeg": self.use_jpeg,
            "source": self.source_mode.as_str(),
            "image_path": self.current_image_path.clone(),
            "topic": self.current_topic.clone(),
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

fn save_raw_image(buffer: &BufferHandle<Ros2Image>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}

fn save_ycbcr422_image(buffer: &BufferHandle<YCbCr422Image>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_ycbcr_444_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}

impl Widget for &mut ImagePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let topic_state = self.robot.topic_list_state();

        ui.horizontal(|ui| {
            self.overlays.combo_box(ui);
            let mut source_mode = self.source_mode;
            ComboBox::from_label("Source")
                .selected_text(source_mode.label())
                .show_ui(ui, |ui| {
                    ui.add_enabled_ui(false, |ui| {
                        ui.selectable_value(
                            &mut source_mode,
                            ImageSourceMode::Topic,
                            "ROS-Z Topic (Unavailable)",
                        );
                    });
                    ui.selectable_value(&mut source_mode, ImageSourceMode::Legacy, "Legacy Path");
                });
            if source_mode != self.source_mode {
                self.source_mode = source_mode;
                self.resubscribe();
            }

            match self.source_mode {
                ImageSourceMode::Topic => {
                    ui.label("Topic");
                    let response = ui.add(TopicCompletionEdit::new(
                        ui.id().with("image-topic"),
                        &topic_state,
                        &mut self.current_topic,
                    ));
                    if response.changed() {
                        self.resubscribe();
                    }
                }
                ImageSourceMode::Legacy => {
                    if ui.checkbox(&mut self.use_jpeg, "JPEG").changed() {
                        self.resubscribe();
                    }

                    ComboBox::from_label("Image")
                        .selected_text(legacy_image_label(&self.current_image_path))
                        .show_ui(ui, |ui| {
                            let mut selectable_item = |value: &str, label: &str| {
                                let is_selected = self.current_image_path == value;

                                if ui.selectable_label(is_selected, label).clicked() {
                                    self.current_image_path = value.to_string();
                                    self.resubscribe();
                                }
                            };

                            selectable_item(DEFAULT_IMAGE_PATH, "Image Left Raw");
                            selectable_item(LEGACY_YCBCR_PATH, "ycbcr422_image");
                        });
                }
            }

            let maybe_timestamp = match &self.image_buffer {
                ImageBuffer::LegacyRaw(buffer) => buffer.get_last_timestamp(),
                ImageBuffer::Jpeg(buffer) => buffer.get_last_timestamp(),
                ImageBuffer::YCbCr(buffer) => buffer.get_last_timestamp(),
                ImageBuffer::UnsupportedTopic => Ok(None),
            };
            if let Ok(Some(timestamp)) = maybe_timestamp {
                let date: DateTime<Utc> = timestamp.as_system_time().into();
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
                        ImageBuffer::LegacyRaw(buffer) => save_raw_image(buffer, path),
                        ImageBuffer::Jpeg(buffer) => {
                            save_jpeg_image(buffer, path.with_extension("jpeg"))
                        }
                        ImageBuffer::YCbCr(buffer) => save_ycbcr422_image(buffer, path),
                        ImageBuffer::UnsupportedTopic => {
                            Err(eyre!(topic_image_unavailable_message()))
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
    fn resubscribe(&mut self) {
        self.image_buffer = subscribe_image(
            &self.robot,
            self.source_mode,
            self.use_jpeg,
            &self.current_image_path,
            &self.current_topic,
        );
    }

    fn load_latest_texture(&self, context: &Context) -> Result<(TextureId, (u32, u32))> {
        let image_identifier = "bytes://image-vision".to_string();
        match &self.image_buffer {
            ImageBuffer::LegacyRaw(buffer) => {
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
            ImageBuffer::UnsupportedTopic => bail!(topic_image_unavailable_message()),
            ImageBuffer::Jpeg(buffer) => {
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
            ImageBuffer::YCbCr(buffer) => {
                let image = buffer
                    .get_last_value()?
                    .ok_or_else(|| eyre!("no image available"))?;
                if image.height() == 0 || image.width() == 0 {
                    bail!(
                        "Image has no pixels. Dimensions: {}x{}",
                        image.width(),
                        image.height()
                    );
                }

                let rgb_image: RgbImage = image.into();

                let image = ColorImage::from_rgb(
                    [rgb_image.width() as usize, rgb_image.height() as usize],
                    rgb_image.as_bytes(),
                );
                let id = context
                    .load_texture(&image_identifier, image, TextureOptions::NEAREST)
                    .id();

                Ok((id, (rgb_image.width(), rgb_image.height())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn image_source_mode_defaults_to_legacy_without_saved_source() {
        assert!(matches!(
            ImageSourceMode::from_value(None),
            ImageSourceMode::Legacy
        ));
    }

    #[test]
    fn image_source_mode_defaults_to_legacy_for_unknown_saved_source() {
        let value = json!({ "source": "future-source" });

        assert!(matches!(
            ImageSourceMode::from_value(Some(&value)),
            ImageSourceMode::Legacy
        ));
    }

    #[test]
    fn unavailable_topic_image_error_mentions_ros_z_topic_images() {
        assert!(topic_image_unavailable_message().contains("ROS-Z topic images are unavailable"));
    }
}
