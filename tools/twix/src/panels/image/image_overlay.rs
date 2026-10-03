use std::{sync::Arc, time::Duration};

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{
    PopupCloseBehavior, Ui,
    containers::menu::{MenuButton, MenuConfig},
};
use ros_z::{Message, time::Time};
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use serde_json::{Map, Value, json};
use twix_visualization::twix_painter::TwixPainter;
use types::time_wrapper::TimeWrapper;

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

use super::overlays::{
    BallDetectionOverlay, FieldBorderOverlay, HorizonOverlay, LineDetectionOverlay,
    ObjectDetectionOverlay, PoseDetectionOverlay,
};

const OVERLAY_RETENTION_WINDOW: Duration = Duration::from_secs(2);

pub(super) struct ImageOverlays {
    line_detection: OverlaySlot<LineDetectionOverlay>,
    ball_detection: OverlaySlot<BallDetectionOverlay>,
    horizon: OverlaySlot<HorizonOverlay>,
    field_border: OverlaySlot<FieldBorderOverlay>,
    object_detection: OverlaySlot<ObjectDetectionOverlay>,
    pose_detection: OverlaySlot<PoseDetectionOverlay>,
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
        }
    }

    pub(super) fn ui<C>(&mut self, ui: &mut Ui, context: &C)
    where
        C: ObservationContext,
    {
        MenuButton::new("Overlays")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.line_detection.checkbox(ui, context);
                self.ball_detection.checkbox(ui, context);
                self.horizon.checkbox(ui, context);
                self.field_border.checkbox(ui, context);
                self.object_detection.checkbox(ui, context);
                self.pose_detection.checkbox(ui, context);
            });
    }

    pub(super) fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        self.line_detection.paint(painter, image_time);
        self.ball_detection.paint(painter, image_time);
        self.horizon.paint(painter, image_time);
        self.field_border.paint(painter, image_time);
        self.object_detection.paint(painter, image_time);
        self.pose_detection.paint(painter, image_time);
    }

    pub(super) fn preferred_image_time(&self) -> Option<Time> {
        [
            self.object_detection.latest_time(),
            self.pose_detection.latest_time(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    pub(super) fn save(&self) -> Value {
        json!({
            LineDetectionOverlay::STORAGE_KEY: self.line_detection.save(),
            BallDetectionOverlay::STORAGE_KEY: self.ball_detection.save(),
            HorizonOverlay::STORAGE_KEY: self.horizon.save(),
            FieldBorderOverlay::STORAGE_KEY: self.field_border.save(),
            ObjectDetectionOverlay::STORAGE_KEY: self.object_detection.save(),
            PoseDetectionOverlay::STORAGE_KEY: self.pose_detection.save(),
        })
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
        }
    }
}

struct OverlaySlot<T> {
    active: bool,
    overlay: Option<T>,
    error: Option<String>,
    settings: Map<String, Value>,
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
        let overlay_value = value.and_then(|value| value.get(T::STORAGE_KEY));
        slot.settings = overlay_value
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        slot.active = slot
            .settings
            .remove("active")
            .and_then(|value| value.as_bool())
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
            settings: Map::new(),
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
                if let Some(overlay) = self.overlay.take() {
                    self.settings = overlay.save();
                }
                self.error = None;
            }
        }
        if let Some(overlay) = &mut self.overlay {
            ui.indent(T::STORAGE_KEY, |ui| overlay.ui(ui));
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    fn recreate<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        match T::new(context, &self.settings) {
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

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        if let Some(overlay) = &self.overlay {
            overlay.paint(painter, image_time);
        }
    }

    fn latest_time(&self) -> Option<Time> {
        self.overlay.as_ref().and_then(ImageOverlay::latest_time)
    }

    fn save(&self) -> Value {
        let mut value = self
            .overlay
            .as_ref()
            .map(ImageOverlay::save)
            .unwrap_or_else(|| self.settings.clone());
        value.insert("active".to_string(), json!(self.active));
        Value::Object(value)
    }
}

pub(super) trait ImageOverlay: Sized {
    const NAME: &'static str;
    const STORAGE_KEY: &'static str;

    fn new<C>(context: &C, settings: &Map<String, Value>) -> Result<Self, Report>
    where
        C: ObservationContext;

    /// Render controls beneath this overlay's selection entry.
    fn ui(&mut self, _ui: &mut Ui) {}

    /// Save overlay settings; the slot stores activation separately.
    fn save(&self) -> Map<String, Value> {
        Map::new()
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time);

    fn latest_time(&self) -> Option<Time> {
        None
    }
}

pub(super) struct OverlayObservation<T> {
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
        let (observation, repaint) = create_typed_observation(context, topic)?;
        Ok(Self {
            observation,
            _repaint: repaint,
        })
    }

    pub(super) fn latest(&self) -> Option<Arc<SampleRecord<T>>> {
        self.observation.latest()
    }

    fn get_all(&self) -> Vec<Arc<SampleRecord<T>>> {
        self.observation.get_all()
    }
}

impl<T> OverlayObservation<TimeWrapper<T>>
where
    TimeWrapper<T>: Message + Send + Sync + 'static,
    <TimeWrapper<T> as Message>::Codec: Send + Sync,
{
    pub(super) fn latest_time(&self) -> Option<Time> {
        self.latest().map(|record| record.value.time)
    }

    pub(super) fn nearest_to_time(
        &self,
        time: Time,
        tolerance: Duration,
    ) -> Option<Arc<SampleRecord<TimeWrapper<T>>>> {
        let nearest = self
            .get_all()
            .into_iter()
            .min_by_key(|record| time_distance(record.value.time, time))?;
        (time_distance(nearest.value.time, time) <= tolerance).then_some(nearest)
    }

    pub(super) fn at_time(&self, time: Time) -> Option<Arc<SampleRecord<TimeWrapper<T>>>> {
        self.get_all()
            .into_iter()
            .rev()
            .find(|record| record.value.time == time)
    }
}

fn time_distance(first: Time, second: Time) -> Duration {
    first
        .duration_since(second)
        .max(second.duration_since(first))
}

fn create_typed_observation<T>(
    context: &impl ObservationContext,
    topic: &str,
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
        .retention(RetentionPolicy::time_window(OVERLAY_RETENTION_WINDOW)?)
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}
