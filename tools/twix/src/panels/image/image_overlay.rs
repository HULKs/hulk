use std::sync::Arc;

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Ui, pos2};
use linear_algebra::{Point2, point};
use ros_z::Message;
use ros_z_debug::{SampleRecord, TopicObservation};
use serde_json::{Value, json};

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

use super::overlays::{
    BallDetectionOverlay, FieldBorderOverlay, HorizonOverlay, LineDetectionOverlay,
    ObjectDetectionOverlay, PoseDetectionOverlay,
};

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
        ui.menu_button("Overlays", |ui| {
            self.line_detection.checkbox(ui, context);
            self.ball_detection.checkbox(ui, context);
            self.horizon.checkbox(ui, context);
            self.field_border.checkbox(ui, context);
            self.object_detection.checkbox(ui, context);
            self.pose_detection.checkbox(ui, context);
        });
    }

    pub(super) fn paint(&self, painter: &ImageOverlayPainter) {
        self.line_detection.paint(painter);
        self.ball_detection.paint(painter);
        self.horizon.paint(painter);
        self.field_border.paint(painter);
        self.object_detection.paint(painter);
        self.pose_detection.paint(painter);
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

    fn paint(&self, painter: &ImageOverlayPainter) {
        if let Some(overlay) = &self.overlay {
            overlay.paint(painter);
        }
    }

    fn save(&self) -> Value {
        json!({"active": self.active})
    }
}

pub(super) trait ImageOverlay: Sized {
    const NAME: &'static str;
    const STORAGE_KEY: &'static str;

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext;

    fn paint(&self, painter: &ImageOverlayPainter);
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
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

pub(super) struct ImageOverlayPainter {
    painter: Painter,
    rect: Rect,
    image_size: [usize; 2],
    scale: f32,
}

impl ImageOverlayPainter {
    pub(super) fn new(painter: Painter, rect: Rect, image_size: [usize; 2]) -> Self {
        let scale_x = rect.width() / image_size[0].max(1) as f32;
        let scale_y = rect.height() / image_size[1].max(1) as f32;
        Self {
            painter,
            rect,
            image_size,
            scale: scale_x.min(scale_y),
        }
    }

    pub(super) fn image_width(&self) -> f32 {
        self.image_size[0] as f32
    }

    fn position(&self, point: Point2<Pixel>) -> Pos2 {
        let scale_x = self.rect.width() / self.image_size[0].max(1) as f32;
        let scale_y = self.rect.height() / self.image_size[1].max(1) as f32;
        pos2(
            self.rect.left() + point.x() * scale_x,
            self.rect.top() + point.y() * scale_y,
        )
    }

    fn stroke(&self, stroke: Stroke) -> Stroke {
        Stroke {
            width: stroke.width * self.scale,
            ..stroke
        }
    }

    pub(super) fn line_segment(&self, start: Point2<Pixel>, end: Point2<Pixel>, stroke: Stroke) {
        self.painter.line_segment(
            [self.position(start), self.position(end)],
            self.stroke(stroke),
        );
    }

    pub(super) fn rect_stroke(&self, min: Point2<Pixel>, max: Point2<Pixel>, stroke: Stroke) {
        let top_right = point![max.x(), min.y()];
        let bottom_left = point![min.x(), max.y()];
        self.line_segment(min, top_right, stroke);
        self.line_segment(top_right, max, stroke);
        self.line_segment(max, bottom_left, stroke);
        self.line_segment(bottom_left, min, stroke);
    }

    pub(super) fn circle_filled(&self, center: Point2<Pixel>, radius: f32, fill_color: Color32) {
        self.painter
            .circle_filled(self.position(center), radius * self.scale, fill_color);
    }

    pub(super) fn circle_stroke(&self, center: Point2<Pixel>, radius: f32, stroke: Stroke) {
        self.painter.circle_stroke(
            self.position(center),
            radius * self.scale,
            self.stroke(stroke),
        );
    }

    pub(super) fn floating_text(
        &self,
        position: Point2<Pixel>,
        align: Align2,
        text: String,
        color: Color32,
    ) {
        self.painter.text(
            self.position(position),
            align,
            text,
            FontId::default(),
            color,
        );
    }
}
