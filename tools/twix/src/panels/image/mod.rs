use std::{sync::Arc, time::Duration};

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions, Ui};
use hulk_widgets::CompletionEdit;
use image::RgbImage;
use linear_algebra::{point, vector};
use ros_z::{Message, entity::EndpointKind, time::Time};
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use ros2::sensor_msgs::image::Image as RosImage;
use serde_json::{Value, json};
use thiserror::Error;
use twix_visualization::{
    twix_painter::{Orientation, TwixPainter},
    zoom_and_pan::ZoomAndPanTransform,
};
use uuid::Uuid;

use crate::{
    graph::TopicCompletionQuery,
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
};

use self::image_overlay::ImageOverlays;

mod image_overlay;
mod overlays;

pub const DEFAULT_IMAGE_TOPIC: &str = "inputs/left_image";
const IMAGE_RETENTION_WINDOW: Duration = Duration::from_secs(2);

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
    overlays: Box<ImageOverlays>,
    zoom_and_pan: ZoomAndPanTransform,
}

enum ObservationState {
    Idle,
    Observing(Box<ObservedImage>),
    Error(String),
}

struct ObservedImage {
    observation: TopicObservation<RosImage>,
    _repaint: ObservationRepaint,
    render_cache: RenderedImageCache,
}

impl Panel for ImagePanel {
    const STORAGE_ID: &'static str = "image";
    const DISPLAY_NAME: &'static str = "Image";
    const ICON: &'static str = egui_material_icons::icons::ICON_PHOTO_CAMERA.codepoint;

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
            overlays: Box::new(ImageOverlays::new(
                context.value.and_then(|value| value.get("overlays")),
                &context,
            )),
            zoom_and_pan: context
                .value
                .and_then(|value| value.get("zoom_and_pan"))
                .and_then(|value| serde_json::from_value::<ZoomAndPanTransform>(value.clone()).ok())
                .unwrap_or_default(),
        };
        panel.recreate_observation(&context);
        panel
    }

    fn header_ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.overlays.ui(ui, &context);
        ui.label("Topic");
        let namespace = context.backend.namespace();
        let completions = {
            let graph = context.backend.graph().lock();
            TopicCompletionQuery::new(&namespace, &self.topic_editor)
                .endpoint_kind(EndpointKind::Publisher)
                .type_name(RosImage::type_name())
                .complete(graph.publishers())
        };
        let response = ui.add(CompletionEdit::new(
            ui.id().with("image_topic"),
            &completions,
            &mut self.topic_editor,
        ));
        if response.changed() {
            self.commit_topic(&context);
        }
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
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
                let preferred_image_time = self.overlays.preferred_image_time();
                if let Some(time) = preferred_image_time {
                    ui.label(format!("{}", format!("{}", time.as_nanos())));
                }
                observed.render_cache.refresh(
                    context.egui_context,
                    &observed.observation,
                    preferred_image_time,
                );

                if let Some(error) = observed.render_cache.error() {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    return;
                }

                let Some(texture) = observed.render_cache.texture() else {
                    ui.label("no data yet");
                    return;
                };

                let [width, height] = observed.render_cache.dimensions().unwrap_or(texture.size());
                let (response, mut painter) = TwixPainter::<Pixel>::allocate(
                    ui,
                    vector![width as f32, height as f32],
                    point![0.0, 0.0],
                    Orientation::LeftHanded,
                );
                self.zoom_and_pan.apply(ui, &mut painter, &response);
                painter.image(
                    texture.id(),
                    geometry::rectangle::Rectangle {
                        min: point![0.0, 0.0],
                        max: point![width as f32, height as f32],
                    },
                );
                self.overlays.paint(
                    &painter,
                    observed
                        .render_cache
                        .image_time()
                        .unwrap_or_else(Time::zero),
                );
                if let Some(position) = response.hover_pos() {
                    let pixel = painter.transform_pixel_to_world(position);
                    response.on_hover_text_at_pointer(format!(
                        "x: {:.1}, y: {:.1}",
                        pixel.x(),
                        pixel.y()
                    ));
                }
            }
        };
    }

    fn save(&self) -> Value {
        json!({
            "topic": self.topic,
            "overlays": self.overlays.save(),
            "zoom_and_pan": serde_json::to_value(&self.zoom_and_pan)
                .expect("failed to serialize image zoom and pan"),
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
}

struct RenderedImageCache {
    sample: Option<Arc<SampleRecord<RosImage>>>,
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
            texture: None,
            dimensions: None,
            error: None,
            texture_name: texture_name.into(),
        }
    }

    fn refresh(
        &mut self,
        egui_context: &Context,
        observation: &TopicObservation<RosImage>,
        preferred_image_time: Option<Time>,
    ) {
        let sample = preferred_image_time
            .and_then(|time| image_sample_at_time(observation, time))
            .or_else(|| observation.latest());
        self.refresh_sample(egui_context, sample);
    }

    fn refresh_sample(
        &mut self,
        egui_context: &Context,
        sample: Option<Arc<SampleRecord<RosImage>>>,
    ) {
        if same_sample(self.sample.as_ref(), sample.as_ref()) {
            return;
        }

        self.sample = sample;
        self.texture = None;
        self.dimensions = None;
        self.error = None;

        let Some(record) = self.sample.as_ref() else {
            return;
        };

        match decode_color_image(&record.value) {
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

    fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    fn dimensions(&self) -> Option<[usize; 2]> {
        self.dimensions
    }

    fn image_time(&self) -> Option<Time> {
        self.sample.as_ref().map(|record| image_time(&record.value))
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

fn same_sample(
    current: Option<&Arc<SampleRecord<RosImage>>>,
    next: Option<&Arc<SampleRecord<RosImage>>>,
) -> bool {
    match (current, next) {
        (Some(current), Some(next)) => Arc::ptr_eq(current, next),
        (None, None) => true,
        _ => false,
    }
}

fn image_time(image: &RosImage) -> Time {
    image.header.stamp.into()
}

fn image_sample_at_time(
    observation: &TopicObservation<RosImage>,
    time: Time,
) -> Option<Arc<SampleRecord<RosImage>>> {
    observation
        .get_all()
        .iter()
        .rev()
        .find(|record| image_time(&record.value) == time)
        .cloned()
}

fn create_observation(
    context: &impl ObservationContext,
    topic: &str,
) -> Result<(TopicObservation<RosImage>, ObservationRepaint), Report> {
    let runtime_handle = context.backend().runtime_handle().clone();
    // ros_z_debug spawns observation tasks internally and needs a current runtime.
    let _runtime_context = runtime_handle.enter();
    let observation = context
        .backend()
        .observer()
        .observe_typed::<RosImage>(topic)
        .wrap_err("failed to create image topic observation")?
        .retention(RetentionPolicy::time_window(IMAGE_RETENTION_WINDOW)?)
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use eframe::egui::Context as EguiContext;
    use ros_z::context::ContextBuilder;
    use ros_z_debug::{TopicObserver, TopicObserverOptions};
    use ros2::{sensor_msgs::image::Image as RosImage, std_msgs::header::Header};

    use super::RenderedImageCache;

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
            .publisher::<RosImage>("/inputs/left_image")
            .build()
            .await
            .unwrap();
        let observation = observer
            .observe_typed::<RosImage>("inputs/left_image")
            .unwrap()
            .spawn();
        let mut cache = RenderedImageCache::new("test-image-cache");
        let image = rgb8_image(2, 1, vec![255, 0, 0, 0, 255, 0]);

        tokio::time::timeout(Duration::from_secs(3), async {
            while observation.latest().is_none() {
                publisher.publish(&image).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("observation should receive published image");

        cache.refresh(&context, &observation, None);

        assert_eq!(cache.dimensions(), Some([2, 1]));
        assert!(cache.texture().is_some());
        assert!(cache.error().is_none());
    }
}
