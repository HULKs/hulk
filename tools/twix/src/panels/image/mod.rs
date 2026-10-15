use crate::{
    graph::TopicCompletionQuery,
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
};
use color_eyre::{Report, eyre::eyre};
use coordinate_systems::Pixel;
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions, Ui};
use hulk_widgets::CompletionEdit;
use image::RgbImage;
use image_overlay::ImageOverlays;
use linear_algebra::{point, vector};
use ros_z::{Message, message::WireDecoder};
use ros_z_debug::{
    TargetIdentity, TopicReference,
    query::{
        Cursor, TimeEncoding, TimeProjection, TypedPlanObservation,
        plan::{self, Frame, Sample, Source, State, TopicSource},
    },
};
use ros2::sensor_msgs::image::Image as RosImage;
use serde_json::{Value, json};
use twix_visualization::{
    twix_painter::{Orientation, TwixPainter},
    zoom_and_pan::ZoomAndPanTransform,
};
use types::stereo_image_pair::StereoImagePair;
use uuid::Uuid;

mod image_overlay;
mod inputs;
mod overlays;
use inputs::SampleIdentity;

pub const DEFAULT_IMAGE_TOPIC: &str = "inputs/left_image";
const STEREO_IMAGE_TOPIC: &str = "inputs/stereo_image_pair";

pub struct ImagePanel {
    topic_editor: String,
    focus_topic_requested: bool,
    topic: String,
    observation: Result<Option<ObservedImage>, String>,
    overlays: ImageOverlays,
    zoom_and_pan: ZoomAndPanTransform,
}

#[derive(PartialEq, Eq)]
struct Binding {
    namespace: String,
    topic: String,
    generation: u64,
    follow: bool,
    overlays: u64,
}

struct ObservedImage {
    observation: TypedPlanObservation<Option<Frame>>,
    binding: Binding,
    _repaint: ObservationRepaint,
    render_cache: RenderedImageCache,
}

impl ImagePanel {
    fn binding(&self, context: &impl ObservationContext) -> Binding {
        Binding {
            namespace: context.backend().namespace(),
            topic: self.topic.clone(),
            generation: context.backend().presentation_identity().0,
            follow: context
                .backend()
                .query_connection()
                .cursor
                .position()
                .cursor
                == Cursor::Live,
            overlays: self.overlays.revision,
        }
    }
    fn recreate(&mut self, context: &impl ObservationContext) {
        let result = (|| -> Result<Option<ObservedImage>, Report> {
            if self.topic.is_empty() {
                self.overlays.clear();
                return Ok(None);
            }
            let binding = self.binding(context);
            let identity = TargetIdentity::new(&binding.namespace)?;
            let topic = TopicReference::new(&self.topic)?.resolve(&identity)?;
            let supported =
                topic == TopicReference::new(DEFAULT_IMAGE_TOPIC)?.resolve(&identity)?;
            let mut sources = vec![TopicSource::new(
                topic,
                TimeProjection::field(".header.stamp", TimeEncoding::RosTime)?,
            )];
            if supported {
                sources.push(TopicSource::new(
                    TopicReference::new(STEREO_IMAGE_TOPIC)?.resolve(&identity)?,
                    TimeProjection::field(".left.header.stamp", TimeEncoding::RosTime)?,
                ));
            }
            let images = Source::first_present(sources);
            let (inputs, preferences) =
                self.overlays.declarations(&binding.namespace, supported)?;
            let primary = if binding.follow && !preferences.is_empty() {
                let time = plan::available(
                    preferences
                        .iter()
                        .map(|source| source.latest(plan::cursor()).effective_time()),
                )
                .min();
                images.exact(time).or_else(images.latest(plan::cursor()))
            } else {
                images.latest(plan::cursor())
            };
            let connection = context.backend().query_connection();
            let _runtime = context.backend().runtime_handle().enter();
            let observation = primary
                .attach(inputs)
                .observe(connection.backend, connection.cursor)?;
            let repaint = observation.raw().repaint_on_updates(context);
            let cache = match &mut self.observation {
                Ok(Some(old))
                    if old.binding.namespace == binding.namespace
                        && old.binding.topic == binding.topic
                        && old.binding.generation == binding.generation =>
                {
                    std::mem::replace(&mut old.render_cache, RenderedImageCache::new())
                }
                _ => RenderedImageCache::new(),
            };
            Ok(Some(ObservedImage {
                observation,
                binding,
                _repaint: repaint,
                render_cache: cache,
            }))
        })();
        self.observation = result.map_err(|error| format!("{error:#}"));
    }
    fn refresh(&mut self, context: &impl ObservationContext) {
        let binding = self.binding(context);
        if self
            .observation
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .is_none_or(|observed| observed.binding != binding)
        {
            self.recreate(context);
        }
        let Ok(Some(observed)) = &mut self.observation else {
            return;
        };
        let snapshot = observed.observation.snapshot();
        match snapshot.state {
            State::Ready(Some(frame)) => {
                if observed
                    .render_cache
                    .refresh(&context.egui_context(), &frame.primary)
                {
                    self.overlays.prepare(&frame);
                }
            }
            State::Ready(None) => {
                observed.render_cache.clear();
                self.overlays.clear();
            }
            State::Failed(error) => observed.render_cache.error = Some(error),
            State::Pending => {}
        }
    }
}

impl Panel for ImagePanel {
    const STORAGE_ID: &'static str = "image";
    const DISPLAY_NAME: &'static str = "Image";
    const ICON: &'static str = egui_material_icons::icons::ICON_IMAGE.codepoint;
    fn new(context: PanelCreationContext<'_>) -> Self {
        let topic = context
            .value
            .and_then(|v| v.get("topic"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_IMAGE_TOPIC)
            .to_owned();
        let mut panel = Self {
            topic_editor: topic.clone(),
            focus_topic_requested: false,
            topic,
            observation: Ok(None),
            overlays: ImageOverlays::new(context.value.and_then(|v| v.get("overlays"))),
            zoom_and_pan: context
                .value
                .and_then(|v| v.get("zoom_and_pan"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
        };
        panel.recreate(&context);
        panel
    }

    fn focus_topic(&mut self) {
        self.focus_topic_requested = true;
    }

    fn header_ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.overlays.ui(ui);
        ui.label("Topic");
        let namespace = context.backend.namespace();
        let completions = {
            let topics = context.backend.topics().snapshot();
            TopicCompletionQuery::new(&namespace, &self.topic_editor)
                .type_name(RosImage::type_name())
                .complete_topics(&topics.topics)
        };
        let response = ui.add(
            CompletionEdit::new(
                ui.id().with("image_topic"),
                &completions,
                &mut self.topic_editor,
            )
            .request_focus(std::mem::take(&mut self.focus_topic_requested)),
        );
        if response.changed() {
            self.topic = self.topic_editor.trim().to_owned();
        }
    }
    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.refresh(&context);
        if self.topic.is_empty() {
            ui.label("Enter an image topic.");
            return;
        }
        let observed = match &self.observation {
            Ok(Some(observed)) => observed,
            Ok(None) => {
                ui.label("No observation.");
                return;
            }
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                return;
            }
        };
        if let Some(error) = &observed.render_cache.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let Some(texture) = &observed.render_cache.texture else {
            ui.label("no data yet");
            return;
        };
        let [width, height] = texture.size();
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
        if let Some(sample) = &observed.render_cache.sample {
            self.overlays.paint(&painter, sample.time);
        }
        if let Some(position) = response.hover_pos() {
            let pixel = painter.transform_pixel_to_world(position);
            response.on_hover_text_at_pointer(format!("x: {:.1}, y: {:.1}", pixel.x(), pixel.y()));
        }
    }
    fn save(&self) -> Value {
        json!({"topic":self.topic,"overlays":self.overlays.save(),"zoom_and_pan":self.zoom_and_pan})
    }
}

fn decode_color_image(sample: &Sample) -> Result<ColorImage, Report> {
    let image = if sample.topic.type_info == RosImage::type_info() {
        <RosImage as Message>::Codec::deserialize(&sample.record.data)?
    } else if sample.topic.type_info == StereoImagePair::type_info() {
        <StereoImagePair as Message>::Codec::deserialize(&sample.record.data)?.left
    } else {
        return Err(eyre!(
            "unsupported image source type {}",
            sample.topic.type_info.name
        ));
    };
    if image.width == 0 || image.height == 0 {
        return Err(eyre!("image has no pixels"));
    }
    let rgb: RgbImage = image.try_into()?;
    Ok(ColorImage::from_rgb(
        [rgb.width() as usize, rgb.height() as usize],
        rgb.as_raw(),
    ))
}

struct RenderedImageCache {
    sample: Option<SampleIdentity<'static>>,
    failed: Option<SampleIdentity<'static>>,
    texture: Option<TextureHandle>,
    error: Option<String>,
    texture_name: String,
}
impl RenderedImageCache {
    fn clear(&mut self) {
        self.sample = None;
        self.failed = None;
        self.texture = None;
        self.error = None;
    }
    fn new() -> Self {
        Self {
            sample: None,
            failed: None,
            texture: None,
            error: None,
            texture_name: format!("twix-image-{}", Uuid::new_v4().simple()),
        }
    }
    fn refresh(&mut self, context: &Context, sample: &Sample) -> bool {
        let identity = SampleIdentity::from(sample);
        if self.sample.as_ref().is_some_and(|old| *old == identity) {
            self.error = None;
            return true;
        }
        if self.failed.as_ref().is_some_and(|old| *old == identity) {
            return false;
        }
        match decode_color_image(sample) {
            Ok(image) => {
                self.sample = Some(identity.into_owned());
                self.failed = None;
                self.error = None;
                self.texture =
                    Some(context.load_texture(&self.texture_name, image, TextureOptions::NEAREST));
                true
            }
            Err(error) => {
                self.failed = Some(identity.into_owned());
                self.error = Some(error.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::message::WireEncoder;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
        },
        time::Duration,
    };
    use types::{
        object_detection::{Object, RobocupObjectLabel},
        time_wrapper::TimeWrapper,
    };

    pub(super) fn raw_sample<T: Message>(
        topic: &str,
        id: u64,
        time: i64,
        source_time: i64,
        value: &T,
    ) -> Sample {
        Sample {
            topic: ros_z_debug::query::TopicInfo {
                name: topic.into(),
                type_info: T::type_info(),
                schema: None,
                error: None,
            },
            record: Arc::new(ros_z_debug::query::RawRecord {
                publication: None,
                id,
                time: ros_z::time::Time::from_nanos(time),
                source_time: ros_z::time::Time::from_nanos(source_time),
                log_time: None,
                data: T::Codec::serialize(value).unwrap().into(),
            }),
        }
    }

    #[test]
    fn image_cache_preserves_good_texture_and_reuses_selected_publication() {
        let sample = |id, width| {
            let mut value = image(id, [255, 0, 0]);
            value.width = width;
            raw_sample("/image", id, id as i64, id as i64, &value)
        };
        let context = Context::default();
        let mut cache = RenderedImageCache::new();
        let good = sample(1, 1);
        let payload = Arc::downgrade(&good.record.data);
        assert!(cache.refresh(&context, &good));
        drop(good);
        assert!(
            payload.upgrade().is_none(),
            "texture cache must retain identity, not the wire payload"
        );
        let texture = cache.texture.as_ref().unwrap().id();
        assert!(cache.refresh(&context, &sample(1, 1)));
        assert_eq!(cache.texture.as_ref().unwrap().id(), texture);
        let bad = sample(2, 0);
        let payload = Arc::downgrade(&bad.record.data);
        assert!(!cache.refresh(&context, &bad));
        drop(bad);
        assert!(
            payload.upgrade().is_none(),
            "failed images must not retain payload buffers either"
        );
        assert_eq!(cache.texture.as_ref().unwrap().id(), texture);
        assert!(cache.error.is_some());
        assert!(cache.refresh(&context, &sample(1, 1)));
        assert!(cache.error.is_none());
    }

    fn image(time: u64, pixel: [u8; 3]) -> RosImage {
        let mut image = RosImage {
            width: 1,
            height: 1,
            step: 3,
            encoding: "rgb8".into(),
            data: pixel.to_vec().into(),
            ..Default::default()
        };
        image.header.stamp = ros_z::time::Time::from_nanos(time as i64)
            .to_wallclock()
            .into();
        image
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sustained_live_plan_presents_matching_overlays_without_cursor_chasing() {
        let namespace = format!("/plan_image_{}", Uuid::new_v4().simple());
        let backend = Arc::new(
            crate::backend::RobotBackend::new(
                tokio::runtime::Handle::current(),
                None,
                namespace.clone(),
            )
            .await
            .unwrap(),
        );
        let node = backend.node();
        let images = node
            .publisher::<StereoImagePair>(&format!("{namespace}/{STEREO_IMAGE_TOPIC}"))
            .build()
            .await
            .unwrap();
        let objects = node
            .publisher::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(&format!(
                "{namespace}/detected_objects"
            ))
            .build()
            .await
            .unwrap();
        let saved =
            json!({"overlays":{"object_detection":{"active":true},"horizon":{"active":true}}});
        let context = PanelCreationContext {
            backend: backend.clone(),
            value: Some(&saved),
            egui_context: Context::default(),
        };
        let mut panel = ImagePanel::new(PanelCreationContext {
            backend: backend.clone(),
            value: Some(&saved),
            egui_context: context.egui_context.clone(),
        });
        let paused = Arc::new(AtomicBool::new(false));
        let published = Arc::new(AtomicU64::new(0));
        let pause = paused.clone();
        let latest = published.clone();
        let publisher = tokio::spawn(async move {
            let mut time = 1u64;
            loop {
                if !pause.load(Ordering::Relaxed) {
                    images
                        .publish(&StereoImagePair {
                            frame_identifier: time as u32,
                            left: image(time, [255, 0, 0]),
                            right: image(time + 1, [0, 255, 0]),
                        })
                        .await
                        .unwrap();
                    if time > 3 {
                        objects
                            .publish(&TimeWrapper {
                                time: ros_z::time::Time::from_nanos((time - 3) as i64),
                                inner: vec![Object::<RobocupObjectLabel>::from([
                                    0., 0., 1., 1., 0.9, 0.,
                                ])],
                            })
                            .await
                            .unwrap();
                    }
                    latest.store(time, Ordering::Relaxed);
                    time += 1;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut matched = 0;
            let mut previous = None;
            while matched < 20 {
                panel.refresh(&context);
                let observed = panel.observation.as_ref().unwrap().as_ref().unwrap();
                if let State::Ready(Some(frame)) = observed.observation.snapshot().state {
                    // The horizon has no camera input: it cannot gate this frame.
                    if let State::Ready(records) = &frame.inputs[1]
                        && let Some(detection) = records.first()
                    {
                        assert_eq!(detection.record.time, frame.primary.record.time);
                        assert_eq!(frame.primary.topic.type_info, StereoImagePair::type_info());
                        assert_eq!(
                            decode_color_image(&frame.primary).unwrap().pixels,
                            [eframe::egui::Color32::RED]
                        );
                        if previous != Some(frame.primary.record.time) {
                            matched += 1;
                            previous = Some(frame.primary.record.time);
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(17)).await;
            }
        })
        .await
        .unwrap();
        paused.store(true, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(30)).await;
        let time = ros_z::time::Time::from_nanos(published.load(Ordering::Relaxed) as i64);
        backend.query_connection().cursor.set(Cursor::At(time));
        panel.refresh(&context);
        let observed = panel.observation.as_ref().unwrap().as_ref().unwrap();
        let State::Ready(Some(frame)) = observed.observation.snapshot().state else {
            panic!("fixed frame missing")
        };
        assert_eq!(frame.primary.record.time, time);
        assert!(matches!(&frame.inputs[1],State::Ready(records) if records.is_empty()));
        let texture = observed.render_cache.texture.as_ref().unwrap().id();
        panel.refresh(&context);
        assert_eq!(
            panel
                .observation
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .render_cache
                .texture
                .as_ref()
                .unwrap()
                .id(),
            texture
        );
        panel.topic.clear();
        panel.refresh(&context);
        assert!(matches!(panel.observation, Ok(None)));
        publisher.abort();
    }
}
