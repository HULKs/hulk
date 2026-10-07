use std::{sync::Arc, time::Duration};

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions, Ui};
use hulk_widgets::CompletionEdit;
use image::RgbImage;
use linear_algebra::{point, vector};
use ros_z::{Message, entity::EndpointKind, time::Time};
use ros_z_debug::{
    RetentionPolicy, SampleRecord, TargetIdentity, TopicObservation, TopicReference,
};
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

use self::image_overlay::{ImageOverlays, OverlaySnapshot};

mod image_overlay;
mod overlays;

pub const DEFAULT_IMAGE_TOPIC: &str = "inputs/left_image";
const DEFAULT_IMAGE_HISTORY_CAPACITY: usize = 256;

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
    replay_revision: u64,
    image_history_capacity: usize,
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
    const ICON: &'static str = egui_material_icons::icons::ICON_IMAGE.codepoint;

    fn new(context: PanelCreationContext<'_>) -> Self {
        let topic = context
            .value
            .and_then(|value| value.get("topic"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_IMAGE_TOPIC)
            .to_string();

        let mut panel = Self {
            replay_revision: context.backend.replay_revision(),
            image_history_capacity: context
                .value
                .and_then(|value| value.get("image_history_capacity"))
                .and_then(Value::as_u64)
                .and_then(|capacity| usize::try_from(capacity).ok())
                .unwrap_or(DEFAULT_IMAGE_HISTORY_CAPACITY)
                .clamp(1, 65536),
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
        ui.label("Image history samples");
        if ui
            .add(eframe::egui::DragValue::new(&mut self.image_history_capacity).range(1..=65536))
            .changed()
            && let ObservationState::Observing(observed) = &mut self.observation
        {
            observed
                .observation
                .set_retention(image_retention(self.image_history_capacity));
        }
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
        if self.replay_revision != context.backend.replay_revision() {
            self.replay_revision = context.backend.replay_revision();
            if let ObservationState::Observing(observed) = &mut self.observation {
                observed.render_cache = RenderedImageCache::for_panel();
            }
        }
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
                let namespace = context.backend.namespace();
                let resolve = |topic: &str| {
                    TopicReference::new(topic)
                        .ok()?
                        .resolve(&TargetIdentity::new(&namespace).ok()?)
                        .ok()
                };
                let resolved_topic = resolve(&self.topic);
                let aligned_camera =
                    resolved_topic.is_some() && resolved_topic == resolve(DEFAULT_IMAGE_TOPIC);
                if !aligned_camera {
                    ui.label("Overlays omitted: producers use inputs/left_image.");
                }
                observed.render_cache.refresh(
                    context.egui_context,
                    &observed.observation,
                    &self.overlays,
                    &namespace,
                    resolved_topic.as_deref(),
                    aligned_camera,
                );

                if let Some(error) = observed.render_cache.error() {
                    ui.colored_label(ui.visuals().error_fg_color, error);
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
                observed.render_cache.overlays.paint(&painter);
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
            "image_history_capacity": self.image_history_capacity,
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

        match create_observation(context, &self.topic, self.image_history_capacity) {
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
    failed_sample: Option<Arc<SampleRecord<RosImage>>>,
    texture: Option<TextureHandle>,
    dimensions: Option<[usize; 2]>,
    error: Option<String>,
    texture_name: String,
    overlays: OverlaySnapshot,
    namespace: String,
    overlay_settings: Value,
    projection_invalidated: bool,
    publisher: Option<ros_z::EndpointGlobalId>,
}

impl RenderedImageCache {
    fn for_panel() -> Self {
        Self::new(format!("twix-image-{}", Uuid::new_v4().simple()))
    }

    fn new(texture_name: impl Into<String>) -> Self {
        Self {
            sample: None,
            failed_sample: None,
            texture: None,
            dimensions: None,
            error: None,
            texture_name: texture_name.into(),
            overlays: OverlaySnapshot::default(),
            namespace: String::new(),
            overlay_settings: Value::Null,
            projection_invalidated: false,
            publisher: None,
        }
    }

    fn refresh(
        &mut self,
        egui_context: &Context,
        observation: &TopicObservation<RosImage>,
        overlays: &ImageOverlays,
        namespace: &str,
        resolved_topic: Option<&str>,
        aligned_camera: bool,
    ) {
        if self.namespace != namespace {
            *self = Self::new(self.texture_name.clone());
            self.namespace = namespace.to_owned();
        }
        let latest = observation
            .latest()
            .filter(|s| Some(s.metadata.resolved_topic.as_str()) == resolved_topic);
        if let Some(latest) = &latest {
            let publisher = Some(latest.publication_id.endpoint_global_id());
            let unstamped = image_time(&latest.value) == Time::zero();
            if self.publisher != publisher {
                *self = Self::new(self.texture_name.clone());
                self.namespace = namespace.to_owned();
                self.publisher = publisher;
            }
            if unstamped {
                self.error = Some("Unsupported image timestamp: header.stamp is zero; coherent overlays require stamped images.".into());
                return;
            }
        }
        let settings = overlays.save();
        if self.overlay_settings != settings {
            self.overlay_settings = settings;
            overlays.retain_enabled(&mut self.overlays);
        }
        let mut images = observation.get_all();
        // A restarted publisher's source clock can precede the retained history window.
        if let Some(latest) = latest
            && !images.iter().any(|s| Arc::ptr_eq(s, &latest))
        {
            images.push(latest);
        }
        images.retain(|s| {
            Some(s.metadata.resolved_topic.as_str()) == resolved_topic
                && Some(s.publication_id.endpoint_global_id()) == self.publisher
                && image_time(&s.value) != Time::zero()
        });
        self.refresh_candidates(egui_context, images, overlays, aligned_camera);
    }

    fn refresh_candidates(
        &mut self,
        egui_context: &Context,
        mut images: Vec<Arc<SampleRecord<RosImage>>>,
        overlays: &ImageOverlays,
        aligned_camera: bool,
    ) {
        if overlays.invalidate_projection(&mut self.overlays) {
            self.projection_invalidated = true;
        }
        if self.projection_invalidated
            && let Some(time) = self.image_time()
            && overlays.restore_projection(&mut self.overlays, time)
        {
            self.projection_invalidated = false;
        }
        if let Some(time) = self.image_time() {
            overlays.enrich_residual(&mut self.overlays, time);
        }
        images.retain(|s| {
            self.image_time()
                .is_none_or(|time| image_time(&s.value) > time)
        });
        images.sort_by_key(|s| std::cmp::Reverse(image_time(&s.value)));
        let prepare = |time| {
            if aligned_camera {
                overlays.prepare(time)
            } else {
                OverlaySnapshot::default()
            }
        };
        let detection_times = aligned_camera.then(|| overlays.detection_times()).flatten();
        for image in &images {
            if detection_times
                .as_ref()
                .is_some_and(|times| !times.contains(&image_time(&image.value)))
            {
                continue;
            }
            let snapshot = prepare(image_time(&image.value));
            if (!aligned_camera || overlays.ready(&snapshot))
                && self.refresh_sample(egui_context, Some(Arc::clone(image)))
            {
                self.overlays = snapshot;
                self.projection_invalidated = false;
                return;
            }
        }
    }

    fn refresh_sample(
        &mut self,
        egui_context: &Context,
        sample: Option<Arc<SampleRecord<RosImage>>>,
    ) -> bool {
        if same_sample(self.sample.as_ref(), sample.as_ref()) {
            return true;
        }
        if same_sample(self.failed_sample.as_ref(), sample.as_ref()) {
            return false;
        }
        let Some(record) = sample.as_ref() else {
            return false;
        };
        match decode_color_image(&record.value) {
            Ok(image) => {
                self.sample = sample;
                self.failed_sample = None;
                self.error = None;
                self.dimensions = Some(image.size);
                self.texture = Some(egui_context.load_texture(
                    &self.texture_name,
                    image,
                    TextureOptions::NEAREST,
                ));
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                self.failed_sample = sample;
                false
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

fn create_observation(
    context: &impl ObservationContext,
    topic: &str,
    capacity: usize,
) -> Result<(TopicObservation<RosImage>, ObservationRepaint), Report> {
    let runtime_handle = context.backend().runtime_handle().clone();
    // ros_z_debug spawns observation tasks internally and needs a current runtime.
    let _runtime_context = runtime_handle.enter();
    let observation = context
        .backend()
        .observer()
        .observe_typed::<RosImage>(topic)
        .wrap_err("failed to create image topic observation")?
        .retention(image_retention(capacity))
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

fn image_retention(capacity: usize) -> RetentionPolicy {
    RetentionPolicy::time_window_with_max_samples(
        Duration::MAX,
        capacity.try_into().expect("positive capacity"),
    )
    .expect("image history capacity must be positive")
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use eframe::egui::Context as EguiContext;
    use ros_z::context::ContextBuilder;
    use ros_z_debug::{TopicObserver, TopicObserverOptions};
    use ros2::{sensor_msgs::image::Image as RosImage, std_msgs::header::Header};

    use super::{ImageOverlays, RenderedImageCache};
    use ros_z::time::Time;

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
    async fn render_cache_selects_newest_and_clears_namespace() {
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
            .retention(super::image_retention(
                super::DEFAULT_IMAGE_HISTORY_CAPACITY,
            ))
            .spawn();
        let mut cache = RenderedImageCache::new("test-image-cache");
        let mut image = rgb8_image(2, 1, vec![255, 0, 0, 0, 255, 0]);
        image.header.stamp.sec = 1;

        tokio::time::timeout(Duration::from_secs(3), async {
            while observation.latest().is_none() {
                publisher.publish(&image).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("observation should receive published image");

        let overlays = ImageOverlays::default();
        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/",
            Some("/inputs/left_image"),
            true,
        );

        assert_eq!(cache.dimensions(), Some([2, 1]));
        assert!(cache.texture().is_some());
        assert!(cache.error().is_none());

        let mut newer = image.clone();
        newer.header.stamp.sec = 2;
        tokio::time::timeout(Duration::from_secs(3), async {
            while super::image_time(&observation.latest().unwrap().value)
                != Time::from_nanos(2_000_000_000)
            {
                publisher.publish(&newer).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("observation should receive newer image");

        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/",
            Some("/inputs/left_image"),
            true,
        );
        assert_eq!(cache.image_time(), Some(Time::from_nanos(2_000_000_000)));
        let pinned = cache.sample.clone().unwrap();
        cache.refresh_sample(&context, Some(Arc::clone(&pinned)));
        assert!(Arc::ptr_eq(cache.sample.as_ref().unwrap(), &pinned));
        use super::image_overlay::tests::publish_until;
        publish_until(&publisher, &image, || {
            observation
                .latest()
                .is_some_and(|s| super::image_time(&s.value) == Time::from_nanos(1_000_000_000))
        })
        .await;
        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/",
            Some("/inputs/left_image"),
            true,
        );
        assert!(
            Arc::ptr_eq(cache.sample.as_ref().unwrap(), &pinned),
            "ordinary out-of-order images cannot reset presentation"
        );
        let replacement = node
            .publisher::<RosImage>("/inputs/left_image")
            .build()
            .await
            .unwrap();
        let old_publisher = pinned.publication_id.endpoint_global_id();
        publish_until(&replacement, &image, || {
            observation
                .latest()
                .is_some_and(|s| s.publication_id.endpoint_global_id() != old_publisher)
        })
        .await;
        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/",
            Some("/inputs/left_image"),
            true,
        );
        assert_eq!(
            cache.image_time(),
            Some(Time::from_nanos(1_000_000_000)),
            "new publisher entity confirms restart and permits timestamp rollback"
        );
        assert_ne!(cache.publisher, Some(old_publisher));
        let pinned = Arc::clone(cache.sample.as_ref().unwrap());
        let texture = cache.texture().unwrap().id();
        let mut unstamped = image.clone();
        unstamped.header.stamp = Default::default();
        publish_until(&replacement, &unstamped, || {
            observation
                .latest()
                .is_some_and(|s| super::image_time(&s.value) == Time::zero())
        })
        .await;
        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/",
            Some("/inputs/left_image"),
            true,
        );
        assert!(Arc::ptr_eq(cache.sample.as_ref().unwrap(), &pinned));
        assert_eq!(cache.texture().unwrap().id(), texture);
        assert!(
            cache
                .error()
                .unwrap()
                .contains("Unsupported image timestamp")
        );
        cache.refresh(
            &context,
            &observation,
            &overlays,
            "/new",
            Some("/new/inputs/left_image"),
            true,
        );
        assert!(cache.sample.is_none());
        assert!(cache.texture().is_none());
    }
}
