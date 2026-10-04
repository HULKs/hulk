use std::{collections::BTreeSet, sync::Arc, time::Duration};

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{Popup, PopupCloseBehavior, Ui};
use projection::camera_matrix::CameraMatrix;
use ros_z::{Message, time::Time};
use ros_z_debug::{
    ObservationPolicy, RetentionPolicy, SampleRecord, TargetIdentity, TopicObservation,
    TopicReference,
};
use serde_json::{Value, json};
use types::time_wrapper::TimeWrapper;

use crate::{
    backend::RobotBackend,
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
};
use twix_visualization::twix_painter::TwixPainter;

use super::overlays::{
    BallDetectionOverlay, FieldBorderOverlay, HorizonOverlay, LineDetectionOverlay,
    ObjectDetectionOverlay, PoseDetectionOverlay, ProjectedFieldLinesOverlay,
};

const OVERLAY_HISTORY_CAPACITY: usize = 4096;

fn overlay_retention() -> RetentionPolicy {
    RetentionPolicy::time_window_with_max_samples(
        Duration::MAX,
        OVERLAY_HISTORY_CAPACITY.try_into().unwrap(),
    )
    .unwrap()
}

pub(super) struct ImageOverlays {
    line_detection: OverlaySlot<LineDetectionOverlay>,
    ball_detection: OverlaySlot<BallDetectionOverlay>,
    horizon: OverlaySlot<HorizonOverlay>,
    field_border: OverlaySlot<FieldBorderOverlay>,
    object_detection: OverlaySlot<ObjectDetectionOverlay>,
    pose_detection: OverlaySlot<PoseDetectionOverlay>,
    projected_field_lines: OverlaySlot<ProjectedFieldLinesOverlay>,
}

impl ImageOverlays {
    pub(super) fn new<C>(value: Option<&Value>, context: &C) -> Self
    where
        C: ObservationContext,
    {
        Self {
            line_detection: OverlaySlot::new(value, context),
            ball_detection: OverlaySlot::new(value, context),
            horizon: OverlaySlot::new(value, context),
            field_border: OverlaySlot::new(value, context),
            object_detection: OverlaySlot::new(value, context),
            pose_detection: OverlaySlot::new(value, context),
            projected_field_lines: OverlaySlot::new(value, context),
        }
    }

    pub(super) fn ui<C>(&mut self, ui: &mut Ui, context: &C)
    where
        C: ObservationContext,
    {
        Popup::menu(&ui.button("Overlays"))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                self.line_detection.checkbox(ui, context);
                self.ball_detection.checkbox(ui, context);
                self.horizon.checkbox(ui, context);
                self.field_border.checkbox(ui, context);
                self.object_detection.checkbox(ui, context);
                self.pose_detection.checkbox(ui, context);
                self.projected_field_lines.checkbox(ui, context);
            });
    }

    pub(super) fn prepare(&self, time: Time) -> OverlaySnapshot {
        OverlaySnapshot {
            objects: self.object_detection.prepare(time),
            poses: self.pose_detection.prepare(time),
            horizon: self.horizon.prepare(time),
            border: self.field_border.prepare(time),
            field: self.projected_field_lines.prepare(time),
            field_unavailable: self
                .projected_field_lines
                .overlay
                .as_ref()
                .is_some_and(|overlay| overlay.unavailable(time)),
        }
    }

    pub(super) fn ready(&self, snapshot: &OverlaySnapshot) -> bool {
        (!self.object_detection.active || snapshot.objects.is_some())
            && (!self.pose_detection.active || snapshot.poses.is_some())
            && (!self.horizon.active || snapshot.horizon.is_some())
            && (!self.field_border.active || snapshot.border.is_some())
            && (!self.projected_field_lines.active
                || snapshot.field.is_some()
                || snapshot.field_unavailable)
    }

    pub(super) fn detection_times(&self) -> Option<BTreeSet<Time>> {
        let objects = self.object_detection.active.then(|| {
            self.object_detection
                .overlay
                .as_ref()
                .map(|o| {
                    o.object_detections
                        .get_all()
                        .iter()
                        .map(|s| s.value.time)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default()
        });
        let poses = self.pose_detection.active.then(|| {
            self.pose_detection
                .overlay
                .as_ref()
                .map(|o| {
                    o.poses
                        .get_all()
                        .iter()
                        .map(|s| s.value.time)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default()
        });
        match (objects, poses) {
            (Some(mut objects), Some(poses)) => {
                objects.retain(|time| poses.contains(time));
                Some(objects)
            }
            (objects, poses) => objects.or(poses),
        }
    }

    pub(super) fn retain_enabled(&self, snapshot: &mut OverlaySnapshot) {
        if !self.object_detection.active {
            snapshot.objects = None;
        }
        if !self.pose_detection.active {
            snapshot.poses = None;
        }
        if !self.horizon.active {
            snapshot.horizon = None;
        }
        if !self.field_border.active {
            snapshot.border = None;
        }
        if !self.projected_field_lines.active {
            snapshot.field = None;
        }
    }

    pub(super) fn restore_projection(&self, snapshot: &mut OverlaySnapshot, time: Time) -> bool {
        snapshot.field = self.projected_field_lines.prepare(time);
        snapshot.field.is_some() || !self.projected_field_lines.active
    }

    pub(super) fn enrich_residual(&self, snapshot: &mut OverlaySnapshot, time: Time) {
        if let Some(overlay) = &self.projected_field_lines.overlay
            && let Some(field) = &mut snapshot.field
        {
            overlay.enrich_residual(field, time);
        }
    }

    pub(super) fn invalidate_projection(&self, snapshot: &mut OverlaySnapshot) -> bool {
        if let Some(overlay) = &self.projected_field_lines.overlay
            && snapshot
                .field
                .as_ref()
                .is_some_and(|field| !overlay.valid(field))
        {
            snapshot.field = None;
            return true;
        }
        false
    }

    pub(super) fn save(&self) -> Value {
        json!({
            LineDetectionOverlay::STORAGE_KEY: self.line_detection.save(),
            BallDetectionOverlay::STORAGE_KEY: self.ball_detection.save(),
            HorizonOverlay::STORAGE_KEY: self.horizon.save(),
            FieldBorderOverlay::STORAGE_KEY: self.field_border.save(),
            ObjectDetectionOverlay::STORAGE_KEY: self.object_detection.save(),
            PoseDetectionOverlay::STORAGE_KEY: self.pose_detection.save(),
            ProjectedFieldLinesOverlay::STORAGE_KEY: self.projected_field_lines.save(),
        })
    }
}

#[derive(Default)]
pub(super) struct OverlaySnapshot {
    objects: Option<<ObjectDetectionOverlay as ImageOverlay>::Sample>,
    poses: Option<<PoseDetectionOverlay as ImageOverlay>::Sample>,
    horizon: Option<<HorizonOverlay as ImageOverlay>::Sample>,
    border: Option<<FieldBorderOverlay as ImageOverlay>::Sample>,
    field: Option<<ProjectedFieldLinesOverlay as ImageOverlay>::Sample>,
    pub(super) field_unavailable: bool,
}

impl OverlaySnapshot {
    pub(super) fn paint(&self, painter: &TwixPainter<Pixel>) {
        if let Some(sample) = &self.field {
            ProjectedFieldLinesOverlay::paint(painter, sample);
        }
        if let Some(sample) = &self.horizon {
            HorizonOverlay::paint(painter, sample);
        }
        if let Some(sample) = &self.border {
            FieldBorderOverlay::paint(painter, sample);
        }
        if let Some(sample) = &self.objects {
            ObjectDetectionOverlay::paint(painter, sample);
        }
        if let Some(sample) = &self.poses {
            PoseDetectionOverlay::paint(painter, sample);
        }
    }
}

impl Default for ImageOverlays {
    fn default() -> Self {
        Self {
            line_detection: OverlaySlot::inactive(),
            ball_detection: OverlaySlot::inactive(),
            horizon: OverlaySlot::inactive(),
            field_border: OverlaySlot::inactive(),
            object_detection: OverlaySlot::inactive(),
            pose_detection: OverlaySlot::inactive(),
            projected_field_lines: OverlaySlot::inactive(),
        }
    }
}

struct OverlaySlot<T> {
    active: bool,
    overlay: Option<T>,
    error: Option<String>,
}

impl<T> OverlaySlot<T>
where
    T: ImageOverlay,
{
    fn new<C>(value: Option<&Value>, context: &C) -> Self
    where
        C: ObservationContext,
    {
        let mut slot = Self::inactive();
        slot.active = value
            .and_then(|value| value.get(T::STORAGE_KEY))
            .and_then(|value| value.get("active"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if slot.active {
            slot.recreate(context);
        }
        slot
    }

    fn inactive() -> Self {
        Self {
            active: false,
            overlay: None,
            error: None,
        }
    }

    fn checkbox<C>(&mut self, ui: &mut Ui, context: &C)
    where
        C: ObservationContext,
    {
        let changed = ui.checkbox(&mut self.active, T::NAME).changed();
        if changed {
            if self.active {
                self.recreate(context);
            } else {
                self.overlay = None;
                self.error = None;
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    fn recreate<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        match T::new(context) {
            Ok(overlay) => {
                self.overlay = Some(overlay);
                self.error = None;
            }
            Err(error) => {
                self.overlay = None;
                self.error = Some(format!("{}: {error:#}", T::NAME));
            }
        }
    }

    fn prepare(&self, time: Time) -> Option<T::Sample> {
        self.overlay.as_ref()?.prepare(time)
    }

    fn save(&self) -> Value {
        json!({"active": self.active})
    }
}

pub(super) trait ImageOverlay: Sized {
    type Sample;
    const NAME: &'static str;
    const STORAGE_KEY: &'static str;

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext;

    fn prepare(&self, time: Time) -> Option<Self::Sample>;
    fn paint(painter: &TwixPainter<Pixel>, sample: &Self::Sample);
}

pub(super) struct OverlayObservation<T> {
    backend: Arc<RobotBackend>,
    topic: TopicReference,
    observation: TopicObservation<T>,
    _repaint: ObservationRepaint,
}

impl<T> OverlayObservation<T>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    pub(super) fn new<C>(context: &C, topic: &str) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Self::with_policy(context, topic, ObservationPolicy::default())
    }

    pub(super) fn with_policy<C>(
        context: &C,
        topic: &str,
        policy: ObservationPolicy,
    ) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        let (observation, repaint) = create_typed_observation(context, topic, policy)?;
        Ok(Self {
            backend: Arc::clone(context.backend()),
            topic: TopicReference::new(topic)?,
            observation,
            _repaint: repaint,
        })
    }

    pub(super) fn latest(&self) -> Option<Arc<SampleRecord<T>>> {
        let topic = self.resolved_topic()?;
        self.observation
            .latest()
            .filter(|record| record.metadata.resolved_topic == topic)
    }

    pub(super) fn get_all(&self) -> Vec<Arc<SampleRecord<T>>> {
        let Some(topic) = self.resolved_topic() else {
            return Vec::new();
        };
        self.observation
            .get_all()
            .into_iter()
            .filter(|record| record.metadata.resolved_topic == topic)
            .collect()
    }

    fn resolved_topic(&self) -> Option<String> {
        // Retargeting is asynchronous; the observer can still expose the previous cache.
        self.topic
            .resolve(&TargetIdentity::new(self.backend.namespace()).ok()?)
            .ok()
    }
}

impl<T> OverlayObservation<TimeWrapper<T>>
where
    TimeWrapper<T>: Message + Send + Sync + 'static,
    <TimeWrapper<T> as Message>::Codec: Send + Sync,
{
    pub(super) fn at_time(&self, time: Time) -> Option<Arc<SampleRecord<TimeWrapper<T>>>> {
        self.get_all()
            .into_iter()
            .rev()
            .find(|record| record.value.time == time)
    }

    pub(super) fn interpolate<R>(
        &self,
        time: Time,
        interpolate: impl FnOnce(&T, &T, f32) -> Option<R>,
    ) -> Option<R> {
        let samples = self.get_all();
        let (before, after, fraction) = bracket(&samples, time)?;
        interpolate(&before.inner, &after.inner, fraction)
    }
}

pub(super) fn bracket<T>(
    samples: &[Arc<SampleRecord<TimeWrapper<T>>>],
    time: Time,
) -> Option<(&TimeWrapper<T>, &TimeWrapper<T>, f32)> {
    let before = samples
        .iter()
        .filter(|s| s.value.time <= time)
        .max_by_key(|s| s.value.time)?;
    if before.value.time == time {
        return Some((&before.value, &before.value, 0.0));
    }
    let after = samples
        .iter()
        .rev()
        .filter(|s| s.value.time >= time)
        .min_by_key(|s| s.value.time)?;
    let gap = after.value.time.duration_since(before.value.time);
    // Brackets, not transport age, bound interpolation; callers reject discontinuities.
    let fraction = if gap.is_zero() {
        0.0
    } else {
        time.duration_since(before.value.time).as_secs_f32() / gap.as_secs_f32()
    };
    Some((&before.value, &after.value, fraction))
}

pub(super) fn valid_intrinsics(intrinsics: &projection::intrinsic::Intrinsic) -> bool {
    intrinsics.focals.iter().all(|v| v.is_finite() && *v > 0.0)
        && intrinsics
            .optical_center
            .inner
            .iter()
            .all(|v| v.is_finite())
}

pub(super) fn interpolate_transform<From, To>(
    a: linear_algebra::Isometry3<From, To>,
    b: linear_algebra::Isometry3<From, To>,
    t: f32,
) -> Option<linear_algebra::Isometry3<From, To>> {
    if !a
        .inner
        .to_homogeneous()
        .iter()
        .chain(b.inner.to_homogeneous().iter())
        .all(|v| v.is_finite())
    {
        return None;
    }
    Some(linear_algebra::Isometry3::wrap(
        a.inner.lerp_slerp(&b.inner, t),
    ))
}

impl OverlayObservation<TimeWrapper<CameraMatrix>> {
    pub(super) fn camera_at(&self, time: Time) -> Option<CameraSample> {
        self.interpolate(time, |a, b, t| {
            // Calibration changes are discontinuities, not motion to interpolate.
            if a.intrinsics != b.intrinsics || a.image_size != b.image_size {
                return None;
            }
            if !valid_intrinsics(&a.intrinsics) {
                return None;
            }
            let ground_to_robot = interpolate_transform(a.ground_to_robot, b.ground_to_robot, t)?;
            let robot_to_head = interpolate_transform(a.robot_to_head, b.robot_to_head, t)?;
            let head_to_camera = interpolate_transform(a.head_to_camera, b.head_to_camera, t)?;
            let robot_to_camera = head_to_camera * robot_to_head;
            Some(CameraSample {
                robot_to_camera,
                intrinsics: a.intrinsics,
                horizon: projection::horizon::Horizon::from_parameters(
                    robot_to_camera * ground_to_robot,
                    &a.intrinsics,
                ),
            })
        })
    }
}

pub(super) struct CameraSample {
    pub(super) robot_to_camera:
        linear_algebra::Isometry3<coordinate_systems::Robot, coordinate_systems::Camera>,
    pub(super) intrinsics: projection::intrinsic::Intrinsic,
    pub(super) horizon: Option<projection::horizon::Horizon>,
}

fn create_typed_observation<T>(
    context: &impl ObservationContext,
    topic: &str,
    policy: ObservationPolicy,
) -> Result<(TopicObservation<T>, ObservationRepaint), Report>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    let runtime_handle = context.backend().runtime_handle().clone();
    // ros_z_debug spawns observation tasks internally and needs a current runtime.
    let _runtime_context = runtime_handle.enter();
    let observation = context
        .backend()
        .observer()
        .observe_typed::<T>(topic)
        .wrap_err_with(|| format!("failed to create typed topic observation for {topic}"))?
        .policy(policy)
        .retention(overlay_retention())
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::panel::PanelCreationContext;
    use linear_algebra::point;
    use projection::intrinsic::Intrinsic;
    use ros_z::context::ContextBuilder;
    use ros_z_debug::{TopicObserver, TopicObserverOptions};

    pub(in crate::panels::image) fn projection_overlays(
        overlay: ProjectedFieldLinesOverlay,
    ) -> ImageOverlays {
        ImageOverlays {
            projected_field_lines: OverlaySlot {
                active: true,
                overlay: Some(overlay),
                error: None,
            },
            ..Default::default()
        }
    }

    pub(in crate::panels::image) fn observation<T>(
        context: &PanelCreationContext<'_>,
        node: Arc<ros_z::node::Node>,
        topic: &str,
    ) -> OverlayObservation<T>
    where
        T: Message + Send + Sync + 'static,
        T::Codec: Send + Sync,
    {
        let observer = TopicObserver::new(
            node,
            TopicObserverOptions::with_namespace(context.backend.namespace()).unwrap(),
        );
        let observation = observer
            .observe_typed::<T>(topic)
            .unwrap()
            .retention(overlay_retention())
            .spawn();
        OverlayObservation {
            backend: Arc::clone(&context.backend),
            topic: TopicReference::new(topic).unwrap(),
            _repaint: observation.repaint_on_updates(context),
            observation,
        }
    }

    pub(in crate::panels::image) async fn publish_until<T>(
        publisher: &ros_z::pubsub::Publisher<T>,
        value: &T,
        mut received: impl FnMut() -> bool,
    ) where
        T: Message + Send + Sync,
        T::Codec: Send + Sync,
    {
        tokio::time::timeout(Duration::from_secs(8), async {
            while !received() {
                publisher.publish(value).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("sample should reach asynchronous observer");
    }

    async fn publish_at_until<T>(
        publisher: &ros_z::pubsub::Publisher<T>,
        value: &T,
        source_time: Time,
        mut received: impl FnMut() -> bool,
    ) where
        T: Message + Send + Sync,
        T::Codec: Send + Sync,
    {
        tokio::time::timeout(Duration::from_secs(8), async {
            while !received() {
                publisher
                    .publish_with_source_time(value, source_time)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("sample should reach asynchronous observer");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn coherent_pipeline_delayed_out_of_order_empty_hold_recovery_and_decode_failure() {
        use super::super::{RenderedImageCache, image_time};
        use ros2::{sensor_msgs::image::Image, std_msgs::header::Header};
        use types::{
            object_detection::{Object, RobocupObjectLabel, YOLOObjectLabel},
            pose_detection::Pose,
        };

        let backend = Arc::new(
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                None,
                "/coherent_image_test".into(),
            )
            .await
            .unwrap(),
        );
        let context = PanelCreationContext {
            backend: Arc::clone(&backend),
            value: None,
            egui_context: eframe::egui::Context::default(),
            render_state: None,
        };
        let ros = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            ros.create_node("coherent_pipeline_publisher")
                .with_namespace("/coherent_image_test")
                .build()
                .await
                .unwrap(),
        );
        let images =
            observation::<Image>(&context, Arc::clone(&node), "inputs/left_image").observation;
        images.set_retention(super::super::image_retention(
            super::super::DEFAULT_IMAGE_HISTORY_CAPACITY,
        ));
        let image_pub = node
            .publisher::<Image>("inputs/left_image")
            .build()
            .await
            .unwrap();
        let object_pub = node
            .publisher::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>("detected_objects")
            .build()
            .await
            .unwrap();
        let pose_pub = node
            .publisher::<TimeWrapper<Vec<Pose<YOLOObjectLabel>>>>("detected_poses")
            .build()
            .await
            .unwrap();
        let mut overlays = ImageOverlays {
            object_detection: OverlaySlot {
                active: true,
                error: None,
                overlay: Some(ObjectDetectionOverlay {
                    object_detections: observation(&context, Arc::clone(&node), "detected_objects"),
                }),
            },
            pose_detection: OverlaySlot {
                active: true,
                error: None,
                overlay: Some(PoseDetectionOverlay {
                    poses: observation(&context, Arc::clone(&node), "detected_poses"),
                }),
            },
            ..Default::default()
        };
        let time = |millis: i64| Time::from_nanos(millis * 1_000_000);
        let image = |millis, valid| Image {
            header: Header {
                stamp: time(millis).to_wallclock().into(),
                ..Default::default()
            },
            width: if valid { 1 } else { 0 },
            height: 1,
            encoding: "rgb8".into(),
            step: 3,
            data: vec![255, 0, 0].into(),
            ..Default::default()
        };
        for millis in [1060, 1000, 1030] {
            publish_until(&image_pub, &image(millis, true), || {
                images
                    .get_all()
                    .iter()
                    .any(|s| image_time(&s.value) == time(millis))
            })
            .await;
        }
        // Empty results are complete, but adjacent timestamps are not the same detection frame.
        for millis in [1061, 1000] {
            publish_until(
                &object_pub,
                &TimeWrapper {
                    time: time(millis),
                    inner: vec![],
                },
                || overlays.prepare(time(millis)).objects.is_some(),
            )
            .await;
        }
        publish_until(
            &pose_pub,
            &TimeWrapper {
                time: time(1000),
                inner: vec![],
            },
            || overlays.prepare(time(1000)).poses.is_some(),
        )
        .await;
        assert!(!overlays.ready(&overlays.prepare(time(1060))));
        assert!(overlays.ready(&overlays.prepare(time(1000))));
        let mut cache = RenderedImageCache::new("coherent-timing-test");
        cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
        assert_eq!(cache.image_time(), Some(time(1000)));
        let held = Arc::clone(cache.sample.as_ref().unwrap());
        let held_objects = Arc::clone(cache.overlays.objects.as_ref().unwrap());
        let held_texture = cache.texture().unwrap().id();
        for next in [2000, 3060, 10000] {
            publish_until(&image_pub, &image(next, true), || {
                images
                    .get_all()
                    .iter()
                    .any(|s| image_time(&s.value) == time(next))
            })
            .await;
            cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
            assert!(Arc::ptr_eq(cache.sample.as_ref().unwrap(), &held));
            assert!(Arc::ptr_eq(
                cache.overlays.objects.as_ref().unwrap(),
                &held_objects
            ));
            assert_eq!(cache.texture().unwrap().id(), held_texture);
        }
        cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
        assert_eq!(cache.image_time(), Some(time(1000)));
        assert!(cache.overlays.objects.is_some());
        assert!(cache.overlays.poses.is_some());
        // Newest complete wins, despite a much newer incomplete image.
        for millis in [1060, 1030] {
            publish_until(
                &object_pub,
                &TimeWrapper {
                    time: time(millis),
                    inner: vec![],
                },
                || overlays.prepare(time(millis)).objects.is_some(),
            )
            .await;
            publish_until(
                &pose_pub,
                &TimeWrapper {
                    time: time(millis),
                    inner: vec![],
                },
                || overlays.prepare(time(millis)).poses.is_some(),
            )
            .await;
            cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
            assert_eq!(cache.image_time(), Some(time(1060)));
        }
        let recovered = Arc::clone(cache.sample.as_ref().unwrap());
        publish_until(&image_pub, &image(1200, false), || {
            images
                .get_all()
                .iter()
                .any(|s| image_time(&s.value) == time(1200))
        })
        .await;
        // No overlays needed: decode still must not destroy the committed snapshot.
        cache.refresh_candidates(
            &context.egui_context,
            images
                .get_all()
                .into_iter()
                .filter(|s| image_time(&s.value) <= time(1200))
                .collect(),
            &overlays,
            false,
        );
        assert!(Arc::ptr_eq(cache.sample.as_ref().unwrap(), &recovered));
        assert!(cache.overlays.objects.is_some());
        assert!(cache.texture().is_some());
        assert!(cache.error().is_some());
        // Eviction / unchanged repaints do not unpin overlay inputs.
        cache.refresh_candidates(&context.egui_context, vec![], &overlays, true);
        assert!(cache.overlays.objects.is_some());
        cache.namespace = context.backend.namespace();
        cache.publisher = Some(
            cache
                .sample
                .as_ref()
                .unwrap()
                .publication_id
                .endpoint_global_id(),
        );
        cache.overlay_settings = overlays.save();
        let pinned_poses = Arc::clone(cache.overlays.poses.as_ref().unwrap());
        overlays.object_detection.active = false;
        let objects = overlays.object_detection.overlay.take();
        cache.refresh(
            &context.egui_context,
            &images,
            &overlays,
            &context.backend.namespace(),
            Some("/coherent_image_test/inputs/left_image"),
            true,
        );
        assert!(
            cache.overlays.objects.is_none(),
            "disabled overlay clears immediately"
        );
        assert!(Arc::ptr_eq(
            cache.overlays.poses.as_ref().unwrap(),
            &pinned_poses
        ));
        cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
        assert_eq!(cache.image_time(), Some(time(1060)));
        overlays.object_detection.active = true;
        overlays.object_detection.overlay = objects;
        cache.refresh(
            &context.egui_context,
            &images,
            &overlays,
            &context.backend.namespace(),
            Some("/coherent_image_test/inputs/left_image"),
            true,
        );
        assert_eq!(cache.image_time(), Some(time(1060)));
        assert!(
            cache.overlays.objects.is_none(),
            "initial enable keeps the current snapshot until a newer complete frame"
        );
        assert!(Arc::ptr_eq(
            cache.overlays.poses.as_ref().unwrap(),
            &pinned_poses
        ));
        cache.refresh(
            &context.egui_context,
            &images,
            &overlays,
            "/changed",
            Some("/changed/inputs/left_image"),
            true,
        );
        assert!(cache.sample.is_none());
        assert!(cache.overlays.objects.is_none());
        assert!(cache.overlays.poses.is_none());
        assert!(cache.texture().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn count_history_handles_common_and_differential_two_second_delays() {
        use super::super::{RenderedImageCache, image_retention, image_time};
        use linear_algebra::Isometry3;
        use ros2::sensor_msgs::image::Image;
        use types::{
            localization::{
                LocalizationEstimate, LocalizationState, LocalizationStatus, PoseEstimate,
            },
            object_detection::{Object, RobocupObjectLabel},
        };

        let backend = Arc::new(
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                None,
                "/image_delay_test".into(),
            )
            .await
            .unwrap(),
        );
        let context = PanelCreationContext {
            backend,
            value: None,
            egui_context: eframe::egui::Context::default(),
            render_state: None,
        };
        let ros = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            ros.create_node("delay_publisher")
                .with_namespace("/image_delay_test")
                .build()
                .await
                .unwrap(),
        );
        let image_pub = node
            .publisher::<Image>("inputs/left_image")
            .build()
            .await
            .unwrap();
        let object_pub = node
            .publisher::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>("detected_objects")
            .build()
            .await
            .unwrap();
        let camera_pub = node
            .publisher::<TimeWrapper<CameraMatrix>>("camera_matrix")
            .build()
            .await
            .unwrap();
        let pose_pub = node
            .publisher::<LocalizationEstimate>("localization/estimate")
            .build()
            .await
            .unwrap();
        let geometry_pub = node
            .publisher::<LocalizationStatus>("localization/status")
            .build()
            .await
            .unwrap();
        let time = |millis: i64| Time::from_nanos(millis * 1_000_000);
        let estimate = |time, field: bool| LocalizationEstimate {
            generation: 0,
            time,
            epoch: 0,
            robot_to_local: PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            },
            robot_to_field: field.then_some(PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            }),
        };
        for (field_index, field) in [false, true].into_iter().enumerate() {
            // Common delay, detections delayed relative to images, then the reverse.
            for (mode, (image_delay, geometry_delay)) in
                [(2000, 2000), (0, 2000), (2000, 0)].into_iter().enumerate()
            {
                let time = |stamp| time(stamp + (field_index * 3 + mode) as i64 * 100_000);
                let image = |stamp, valid| {
                    let mut image = Image {
                        width: u32::from(valid),
                        height: 1,
                        encoding: "rgb8".into(),
                        step: 3,
                        data: vec![255, 0, 0].into(),
                        ..Default::default()
                    };
                    image.header.stamp = time(stamp).to_wallclock().into();
                    image
                };
                let images = observation::<Image>(&context, Arc::clone(&node), "inputs/left_image")
                    .observation;
                images.set_retention(image_retention(
                    super::super::DEFAULT_IMAGE_HISTORY_CAPACITY,
                ));
                let mut overlays = if field {
                    projection_overlays(ProjectedFieldLinesOverlay::for_test(
                        &context,
                        Arc::clone(&node),
                    ))
                } else {
                    ImageOverlays::default()
                };
                overlays.object_detection = OverlaySlot {
                    active: true,
                    error: None,
                    overlay: Some(ObjectDetectionOverlay {
                        object_detections: observation(
                            &context,
                            Arc::clone(&node),
                            "detected_objects",
                        ),
                    }),
                };
                let mut cache = RenderedImageCache::new("transport-delay");
                for geometry_first in [image_delay > geometry_delay, image_delay <= geometry_delay]
                {
                    if geometry_first {
                        for stamp in [1000, 1100, 3301] {
                            // Controlled publication source times exercise real observer retention,
                            // not just payload matching. No two-second wall-clock sleep is needed.
                            tokio::time::timeout(Duration::from_secs(8), async {
                                while !overlays.ready(&overlays.prepare(time(stamp))) {
                                    let source = time(stamp + geometry_delay);
                                    camera_pub.publish_with_source_time(&TimeWrapper { time: time(stamp), inner: CameraMatrix::default() }, source).await.unwrap();
                                    pose_pub.publish_with_source_time(&estimate(time(stamp), true), source).await.unwrap();
                                    geometry_pub.publish_with_source_time(&LocalizationStatus { time: time(stamp), epoch: 0, generation: 0, state: LocalizationState::Tracking, heading: None }, source).await.unwrap();
                                    object_pub.publish_with_source_time(&TimeWrapper { time: time(stamp), inner: vec![] }, source).await.unwrap();
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }
                            }).await.unwrap_or_else(|error| panic!("field={field} mode={mode} stamp={stamp}: {error}; objects={}, field={}", overlays.prepare(time(stamp)).objects.is_some(), overlays.prepare(time(stamp)).field.is_some()));
                        }
                    } else {
                        for stamp in [1000, 1100, 3300] {
                            publish_at_until(
                                &image_pub,
                                &image(stamp, true),
                                time(stamp + image_delay),
                                || {
                                    images
                                        .get_all()
                                        .iter()
                                        .any(|s| image_time(&s.value) == time(stamp))
                                },
                            )
                            .await;
                        }
                    }
                }
                assert!(
                    images
                        .get_all()
                        .iter()
                        .any(|s| image_time(&s.value) == time(1000))
                );
                assert!(overlays.ready(&overlays.prepare(time(1000))));
                cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
                assert_eq!(
                    cache.image_time(),
                    Some(time(1100)),
                    "newest complete skips backlog, not the two-second-old result"
                );
                assert_eq!(cache.overlays.field.is_some(), field);
                let texture = cache.texture().unwrap().id();
                let objects = Arc::clone(cache.overlays.objects.as_ref().unwrap());
                images.set_retention(image_retention(1));
                publish_at_until(
                    &image_pub,
                    &image(3302, true),
                    time(3302 + image_delay),
                    || {
                        let history = images.get_all();
                        history.len() == 1 && image_time(&history[0].value) == time(3302)
                    },
                )
                .await;
                cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
                assert_eq!(cache.texture().unwrap().id(), texture);
                assert!(Arc::ptr_eq(
                    cache.overlays.objects.as_ref().unwrap(),
                    &objects
                ));
                assert_eq!(cache.overlays.field.is_some(), field);
                assert!(
                    cache.sample.is_some(),
                    "pending cannot replace the displayed image with waiting text"
                );

                images.set_retention(image_retention(256));
                for (stamp, valid) in [(3400, true), (3500, false)] {
                    publish_at_until(
                        &image_pub,
                        &image(stamp, valid),
                        time(stamp + image_delay),
                        || {
                            images
                                .get_all()
                                .iter()
                                .any(|s| image_time(&s.value) == time(stamp))
                        },
                    )
                    .await;
                    // Explicit None makes projection unavailable, not permanently pending.
                    publish_at_until(
                        &pose_pub,
                        &estimate(time(stamp), false),
                        time(stamp + geometry_delay),
                        || !field || overlays.prepare(time(stamp)).field_unavailable,
                    )
                    .await;
                    publish_at_until(
                        &object_pub,
                        &TimeWrapper {
                            time: time(stamp),
                            inner: vec![],
                        },
                        time(stamp + geometry_delay),
                        || overlays.prepare(time(stamp)).objects.is_some(),
                    )
                    .await;
                }
                cache.refresh_candidates(&context.egui_context, images.get_all(), &overlays, true);
                assert_eq!(
                    cache.image_time(),
                    Some(time(3400)),
                    "malformed newest cannot block a good complete candidate"
                );
                assert!(cache.overlays.objects.is_some());
                assert!(cache.overlays.field.is_none());
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn namespace_filter_rejects_frozen_history_but_keeps_absolute_topics() {
        let backend = Arc::new(
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                None,
                "/overlay_old".to_string(),
            )
            .await
            .unwrap(),
        );
        let context = PanelCreationContext {
            backend: Arc::clone(&backend),
            value: None,
            egui_context: eframe::egui::Context::default(),
            render_state: None,
        };
        let ros = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            ros.create_node("overlay_namespace_test")
                .build()
                .await
                .unwrap(),
        );
        // Use one node for transport, and keep its observer frozen on the old namespace.
        let observer = TopicObserver::new(
            Arc::clone(&node),
            TopicObserverOptions::with_namespace("/overlay_old").unwrap(),
        );
        let observe = |topic: &str| {
            let observation = observer
                .observe_typed::<TimeWrapper<Intrinsic>>(topic)
                .unwrap()
                .retention(overlay_retention())
                .spawn();
            OverlayObservation {
                backend: Arc::clone(&backend),
                topic: TopicReference::new(topic).unwrap(),
                _repaint: observation.repaint_on_updates(&context),
                observation,
            }
        };
        let relative = observe("intrinsics");
        let absolute = observe("/overlay_old/intrinsics");
        let publisher = node
            .publisher::<TimeWrapper<Intrinsic>>("/overlay_old/intrinsics")
            .build()
            .await
            .unwrap();
        let sample = TimeWrapper {
            time: Time::from_nanos(1_000_000_000),
            inner: Intrinsic::default(),
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while relative.latest().is_none() || absolute.latest().is_none() {
                publisher.publish(&sample).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("both observers should receive the sample");
        assert!(relative.at_time(sample.time).is_some());
        let corrected = TimeWrapper {
            time: sample.time,
            inner: Intrinsic::new(nalgebra::vector![2.0, 3.0], point![4.0, 5.0]),
        };
        publish_until(&publisher, &corrected, || {
            relative
                .at_time(sample.time)
                .is_some_and(|s| s.value.inner == corrected.inner)
        })
        .await;
        let history = relative.get_all();
        let (before, after, fraction) = bracket(&history, sample.time).unwrap();
        assert!(
            std::ptr::eq(before, after),
            "exact duplicate stamps must use one winning sample"
        );
        assert_eq!(before.inner, corrected.inner);
        assert_eq!(fraction, 0.0);
        let later = TimeWrapper {
            time: Time::from_nanos(1_100_000_000),
            inner: Intrinsic::default(),
        };
        publish_until(&publisher, &later, || {
            relative.at_time(later.time).is_some()
        })
        .await;
        let corrected_later = TimeWrapper {
            time: later.time,
            inner: corrected.inner,
        };
        publish_until(&publisher, &corrected_later, || {
            relative
                .at_time(later.time)
                .is_some_and(|s| s.value.inner == corrected.inner)
        })
        .await;
        let history = relative.get_all();
        let (before, after, fraction) = bracket(&history, Time::from_nanos(1_050_000_000)).unwrap();
        assert_eq!(before.inner, corrected.inner);
        assert_eq!(after.inner, corrected.inner);
        assert_eq!(fraction, 0.5);
        for nanos in [900_000_000, 1_200_000_000] {
            assert!(relative.at_time(Time::from_nanos(nanos)).is_none());
        }

        backend.set_namespace("/overlay_new".to_string()).unwrap();
        assert!(relative.observation.latest().is_some());
        assert!(relative.latest().is_none());
        assert!(relative.get_all().is_empty());
        assert!(relative.at_time(sample.time).is_none());
        assert!(absolute.latest().is_some());
        assert!(absolute.at_time(sample.time).is_some());
    }
}
