use std::{env::temp_dir, fs::create_dir_all, path::PathBuf, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use color_eyre::{
    Result,
    eyre::{bail, eyre},
};
use eframe::egui::{ColorImage, Context, Response, TextureId, TextureOptions, Ui, Widget};
use geometry::rectangle::Rectangle;
use image::{EncodableLayout, RgbImage};
use linear_algebra::{point, vector};
use log::{info, warn};
use ros2::sensor_msgs::image::Image as Ros2Image;
use serde_json::{Value, json};

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
    TopicRaw(BufferHandle<Ros2Image>),
}

pub struct ImagePanel {
    robot: Arc<Robot>,
    image_buffer: ImageBuffer,
    overlays: Overlays,
    zoom_and_pan: ZoomAndPanTransform,
    current_topic: String,
}

const DEFAULT_TOPIC: &str = "/alpha/robot_hw/left_image";

fn image_topic_from_value(value: Option<&Value>) -> &str {
    value
        .and_then(|value| value.get("topic"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_TOPIC)
}

fn saved_image_config(topic: &str, overlays: Value) -> Value {
    json!({
        "topic": topic,
        "overlays": overlays,
    })
}

fn subscribe_image(robot: &Arc<Robot>, topic: &str) -> ImageBuffer {
    ImageBuffer::TopicRaw(robot.subscribe_topic_value(topic, Duration::ZERO))
}

impl<'a> Panel<'a> for ImagePanel {
    const NAME: &'static str = "Image";

    fn new(context: PanelCreationContext) -> Self {
        let current_topic = image_topic_from_value(context.value).to_string();
        let image_buffer = subscribe_image(&context.robot, &current_topic);

        let overlays = Overlays::new(
            context.robot.clone(),
            context.value.and_then(|value| value.get("overlays")),
        );
        Self {
            robot: context.robot,
            image_buffer,
            overlays,
            zoom_and_pan: ZoomAndPanTransform::default(),
            current_topic,
        }
    }

    fn save(&self) -> Value {
        saved_image_config(&self.current_topic, self.overlays.save())
    }
}

fn save_raw_image(buffer: &BufferHandle<Ros2Image>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}

impl Widget for &mut ImagePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let topic_state = self.robot.topic_list_state();

        ui.horizontal(|ui| {
            self.overlays.combo_box(ui);
            ui.label("Topic");
            let response = ui.add(TopicCompletionEdit::new(
                ui.id().with("image-topic"),
                &topic_state,
                &mut self.current_topic,
            ));
            if response.changed() {
                self.resubscribe();
            }

            let maybe_timestamp = match &self.image_buffer {
                ImageBuffer::TopicRaw(buffer) => buffer.get_last_timestamp(),
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
                        ImageBuffer::TopicRaw(buffer) => save_raw_image(buffer, path),
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
        self.image_buffer = subscribe_image(&self.robot, &self.current_topic);
    }

    fn load_latest_texture(&self, context: &Context) -> Result<(TextureId, (u32, u32))> {
        let image_identifier = "bytes://image-vision".to_string();
        match &self.image_buffer {
            ImageBuffer::TopicRaw(buffer) => {
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
                    .map_err(|error: image::ImageError| eyre!(error))?;

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
    fn image_topic_from_value_uses_default_without_saved_topic() {
        assert_eq!(image_topic_from_value(None), DEFAULT_TOPIC);
    }

    #[test]
    fn image_topic_from_value_uses_saved_topic() {
        let value = json!({ "topic": "/camera/front/image" });

        assert_eq!(image_topic_from_value(Some(&value)), "/camera/front/image");
    }

    #[test]
    fn image_save_keeps_topic_and_overlays_only() {
        let overlays = json!([{ "name": "Horizon", "enabled": true }]);
        let saved = saved_image_config("/camera/front/image", overlays.clone());

        assert_eq!(
            saved,
            json!({
                "topic": "/camera/front/image",
                "overlays": overlays,
            })
        );
    }
}
