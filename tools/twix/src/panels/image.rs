use std::sync::Arc;

use color_eyre::{Report, eyre::Context as _};
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions, Ui, load::SizedTexture};
use hulk_widgets::CompletionEdit;
use image::RgbImage;
use ros_z::{pubsub::PublicationId, time::Time};
use ros_z_debug::{SampleRecord, TopicObservation, TopicObservationStatus};
use ros2::sensor_msgs::image::Image as RosImage;
use serde_json::{Value, json};
use thiserror::Error;
use types::time_wrapper::TimeWrapper;
use uuid::Uuid;

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
};

pub const DEFAULT_IMAGE_TOPIC: &str = "inputs/left_image";

#[derive(Debug, Error)]
enum ImageDecodeError {
    #[error("image has no pixels. Dimensions: {width}x{height}")]
    Empty { width: u32, height: u32 },
    #[error("failed to decode image: {0}")]
    Decode(#[from] image::ImageError),
}

fn decode_color_image(image: &RosImage) -> Result<ColorImage, ImageDecodeError> {
    if image.width == 0 || image.height == 0 {
        return Err(ImageDecodeError::Empty {
            width: image.width,
            height: image.height,
        });
    }

    let rgb_image: RgbImage = image.clone().try_into()?;
    Ok(ColorImage::from_rgb(
        [rgb_image.width() as usize, rgb_image.height() as usize],
        rgb_image.as_raw(),
    ))
}

pub struct ImagePanel {
    topic_editor: String,
    topic: String,
    observation: ObservationState,
}

enum ObservationState {
    Idle,
    Observing(Box<ObservedImage>),
    Error(String),
}

struct ObservedImage {
    observation: TopicObservation<TimeWrapper<RosImage>>,
    _repaint: ObservationRepaint,
    render_cache: RenderedImageCache,
}

struct RenderedMetadata {
    resolved_topic: String,
    type_name: String,
    source_time: String,
    transport_time: String,
    publication_id: String,
    image_time: String,
}

impl Panel for ImagePanel {
    const STORAGE_ID: &'static str = "image";
    const DISPLAY_NAME: &'static str = "Image";

    fn new(context: PanelCreationContext<'_>) -> Self {
        let topic = context
            .value
            .and_then(|value| value.get("topic"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_IMAGE_TOPIC)
            .to_string();

        let mut panel = Self {
            topic_editor: topic.clone(),
            topic,
            observation: ObservationState::Idle,
        };
        panel.recreate_observation(&context);
        panel
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Topic");
                let completions = Vec::<String>::new();
                let response = ui.add(CompletionEdit::new(
                    ui.id().with("image_topic"),
                    &completions,
                    &mut self.topic_editor,
                ));
                if response.changed() {
                    self.commit_topic(&context);
                }
            });

            if self.topic.is_empty() {
                ui.label("Enter an image topic.");
                return;
            }

            match &mut self.observation {
                ObservationState::Idle => {
                    ui.label("No observation.");
                }
                ObservationState::Error(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ObservationState::Observing(observed) => {
                    Self::render_status(ui, observed.observation.status());
                    observed
                        .render_cache
                        .refresh(&context.egui_context, &observed.observation);

                    let Some(metadata) = observed.render_cache.metadata() else {
                        ui.label("Waiting for first sample.");
                        return;
                    };
                    Self::render_metadata(ui, metadata);
                    ui.separator();

                    if let Some(error) = observed.render_cache.error() {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                        return;
                    }

                    if let Some(texture) = observed.render_cache.texture() {
                        let size = observed
                            .render_cache
                            .dimensions()
                            .map(|[width, height]| eframe::egui::vec2(width as f32, height as f32))
                            .unwrap_or_else(|| texture.size_vec2());
                        let texture = SizedTexture {
                            id: texture.id(),
                            size,
                        };
                        ui.add(eframe::egui::Image::new(texture).shrink_to_fit());
                    }
                }
            };
        });
    }

    fn save(&self) -> Value {
        json!({
            "topic": self.topic,
        })
    }
}

impl ImagePanel {
    fn recreate_observation<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        self.observation = ObservationState::Idle;

        if self.topic.is_empty() {
            return;
        }

        match create_observation(context, &self.topic) {
            Ok((observation, repaint)) => {
                self.observation = ObservationState::Observing(Box::new(ObservedImage {
                    observation,
                    _repaint: repaint,
                    render_cache: RenderedImageCache::for_panel(),
                }));
            }
            Err(error) => {
                self.observation = ObservationState::Error(format!("{error:#}"));
            }
        }
    }

    fn commit_topic<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        let next_topic = self.topic_editor.trim().to_string();
        if next_topic == self.topic {
            return;
        }
        self.topic = next_topic;
        self.recreate_observation(context);
    }

    fn render_metadata(ui: &mut Ui, metadata: &RenderedMetadata) {
        ui.horizontal_wrapped(|ui| {
            ui.label("topic:");
            ui.monospace(&metadata.resolved_topic);
            ui.separator();
            ui.label("type:");
            ui.monospace(&metadata.type_name);
            ui.separator();
            ui.label("source:");
            ui.monospace(&metadata.source_time);
            ui.separator();
            ui.label("transport:");
            ui.monospace(&metadata.transport_time);
            ui.separator();
            ui.label("publication:");
            ui.monospace(&metadata.publication_id);
            ui.separator();
            ui.label("image:");
            ui.monospace(&metadata.image_time);
        });
    }

    fn render_status(ui: &mut Ui, status: TopicObservationStatus) {
        ui.horizontal_wrapped(|ui| {
            ui.label("status:");
            ui.monospace(format!("{status:?}"));
        });
    }
}

struct RenderedImageCache {
    sample: Option<Arc<SampleRecord<TimeWrapper<RosImage>>>>,
    metadata: Option<RenderedMetadata>,
    texture: Option<TextureHandle>,
    dimensions: Option<[usize; 2]>,
    error: Option<String>,
    texture_name: String,
}

impl RenderedImageCache {
    fn for_panel() -> Self {
        Self::new(format!("twix-image-{}", Uuid::new_v4().simple()))
    }

    fn new(texture_name: impl Into<String>) -> Self {
        Self {
            sample: None,
            metadata: None,
            texture: None,
            dimensions: None,
            error: None,
            texture_name: texture_name.into(),
        }
    }

    fn refresh(
        &mut self,
        egui_context: &Context,
        observation: &TopicObservation<TimeWrapper<RosImage>>,
    ) {
        self.refresh_sample(egui_context, observation.latest());
    }

    fn refresh_sample(
        &mut self,
        egui_context: &Context,
        sample: Option<Arc<SampleRecord<TimeWrapper<RosImage>>>>,
    ) {
        if same_sample(self.sample.as_ref(), sample.as_ref()) {
            return;
        }

        self.sample = sample;
        self.metadata = None;
        self.texture = None;
        self.dimensions = None;
        self.error = None;

        let Some(record) = self.sample.as_ref() else {
            return;
        };

        self.metadata = Some(RenderedMetadata::from(record.as_ref()));
        match decode_color_image(&record.value.inner) {
            Ok(image) => {
                self.dimensions = Some(image.size);
                self.texture = Some(egui_context.load_texture(
                    &self.texture_name,
                    image,
                    TextureOptions::NEAREST,
                ));
            }
            Err(error) => {
                self.error = Some(error.to_string());
            }
        }
    }

    fn metadata(&self) -> Option<&RenderedMetadata> {
        self.metadata.as_ref()
    }

    fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    fn dimensions(&self) -> Option<[usize; 2]> {
        self.dimensions
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

fn same_sample(
    current: Option<&Arc<SampleRecord<TimeWrapper<RosImage>>>>,
    next: Option<&Arc<SampleRecord<TimeWrapper<RosImage>>>>,
) -> bool {
    match (current, next) {
        (Some(current), Some(next)) => Arc::ptr_eq(current, next),
        (None, None) => true,
        _ => false,
    }
}

impl From<&SampleRecord<TimeWrapper<RosImage>>> for RenderedMetadata {
    fn from(record: &SampleRecord<TimeWrapper<RosImage>>) -> Self {
        Self {
            resolved_topic: record.metadata.resolved_topic.clone(),
            type_name: record.metadata.type_info.name.to_string(),
            source_time: format_time(record.source_time),
            transport_time: record
                .transport_time
                .map(format_time)
                .unwrap_or_else(|| "none".to_string()),
            publication_id: format_publication_id(record.publication_id),
            image_time: format_time(record.value.time),
        }
    }
}

fn create_observation(
    context: &impl ObservationContext,
    topic: &str,
) -> Result<(TopicObservation<TimeWrapper<RosImage>>, ObservationRepaint), Report> {
    let runtime_handle = context.backend().runtime_handle().clone();
    // ros_z_debug spawns observation tasks internally and needs a current runtime.
    let _runtime_context = runtime_handle.enter();
    let observation = context
        .backend()
        .observer()
        .observe_typed::<TimeWrapper<RosImage>>(topic)
        .wrap_err("failed to create image topic observation")?
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

fn format_time(time: Time) -> String {
    format!("{} ns", time.as_nanos())
}

fn format_publication_id(publication_id: PublicationId) -> String {
    format!("{publication_id:#}")
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use eframe::egui::Color32;
    use eframe::egui::Context as EguiContext;
    use ros_z::{EndpointGlobalId, context::ContextBuilder, pubsub::Received, time::Time};
    use ros_z_debug::{TopicObserver, TopicObserverOptions};
    use ros2::{sensor_msgs::image::Image as RosImage, std_msgs::header::Header};
    use serde_json::json;
    use types::time_wrapper::TimeWrapper;

    use crate::{backend::RobotBackend, panel::PanelCreationContext};

    use super::{
        DEFAULT_IMAGE_TOPIC, ImageDecodeError, ImagePanel, ObservationState, RenderedImageCache,
        decode_color_image, format_publication_id,
    };
    use crate::panel::Panel;

    fn publication_id() -> ros_z::pubsub::PublicationId {
        Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 42,
            source_global_id: EndpointGlobalId::from([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ]),
        }
        .publication_id()
    }

    fn rgb8_image(width: u32, height: u32, data: Vec<u8>) -> RosImage {
        RosImage {
            header: Header::default(),
            height,
            width,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: width * 3,
            data: data.into(),
        }
    }

    fn wrapped_image(image: RosImage) -> TimeWrapper<RosImage> {
        TimeWrapper {
            time: Time::from_nanos(123),
            inner: image,
        }
    }

    #[test]
    fn decode_rgb8_image_reports_dimensions_and_pixels() {
        let image = rgb8_image(2, 1, vec![255, 0, 0, 0, 255, 0]);

        let decoded = decode_color_image(&image).unwrap();

        assert_eq!(decoded.size, [2, 1]);
        assert_eq!(decoded.pixels, vec![Color32::RED, Color32::GREEN]);
    }

    #[test]
    fn decode_zero_sized_image_returns_error() {
        let image = rgb8_image(0, 1, vec![]);

        let error = decode_color_image(&image).unwrap_err();

        assert!(matches!(
            error,
            ImageDecodeError::Empty {
                width: 0,
                height: 1
            }
        ));
    }

    #[test]
    fn metadata_formats_compact_publication_id() {
        assert_eq!(
            format_publication_id(publication_id()),
            "01020304…0d0e0f10#42"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn render_cache_decodes_new_rgb8_sample() {
        let context = EguiContext::default();
        let ros_context = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            ros_context
                .create_node("render_cache_decodes_new_rgb8_sample")
                .build()
                .await
                .unwrap(),
        );
        let observer = TopicObserver::new(
            Arc::clone(&node),
            TopicObserverOptions::with_namespace("/").unwrap(),
        );
        let publisher = node
            .publisher::<TimeWrapper<RosImage>>("/inputs/left_image")
            .build()
            .await
            .unwrap();
        let observation = observer
            .observe_typed::<TimeWrapper<RosImage>>("inputs/left_image")
            .unwrap()
            .spawn();
        let mut cache = RenderedImageCache::new("test-image-cache");
        let image = wrapped_image(rgb8_image(2, 1, vec![255, 0, 0, 0, 255, 0]));

        tokio::time::timeout(Duration::from_secs(3), async {
            while observation.latest().is_none() {
                publisher.publish(&image).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("observation should receive published image");

        cache.refresh(&context, &observation);

        assert_eq!(cache.dimensions(), Some([2, 1]));
        assert!(cache.texture().is_some());
        assert!(cache.error().is_none());
    }

    #[test]
    fn save_preserves_topic() {
        let panel = ImagePanel {
            topic_editor: "inputs/right_image".to_string(),
            topic: "inputs/right_image".to_string(),
            observation: ObservationState::Idle,
        };

        assert_eq!(
            panel.save(),
            json!({
                "topic": "inputs/right_image",
            })
        );
    }

    #[test]
    fn new_defaults_to_left_image_without_current_tokio_runtime() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let backend = Arc::new(
            runtime
                .block_on(RobotBackend::new(
                    runtime.handle().clone(),
                    None,
                    "/".to_string(),
                ))
                .expect("backend should build"),
        );

        let panel = ImagePanel::new(PanelCreationContext {
            backend,
            value: None,
            egui_context: EguiContext::default(),
        });

        assert_eq!(panel.topic, DEFAULT_IMAGE_TOPIC);
    }
}
