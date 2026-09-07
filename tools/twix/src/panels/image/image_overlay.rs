use std::{sync::Arc, time::Duration};

use color_eyre::{Report, eyre::Context as _};
use coordinate_systems::Pixel;
use eframe::egui::{
    Align2, Color32, CornerRadius, DragValue, FontId, Mesh, Painter, PopupCloseBehavior, Pos2,
    Rect, Shape, Stroke, StrokeKind, Ui, Vec2,
    containers::menu::{MenuButton, MenuConfig},
    pos2, vec2,
};
use linear_algebra::Point2;
use ros_z::{Message, time::Time};
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use serde_json::{Value, json};
use types::{bounding_box::BoundingBox, time_wrapper::TimeWrapper};

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

use super::overlays::{
    BallDetectionOverlay, FieldBorderOverlay, HorizonOverlay, LineDetectionOverlay,
    ObjectDetectionOverlay, PoseDetectionOverlay,
};

const OVERLAY_RETENTION_WINDOW: Duration = Duration::from_secs(2);
const DETECTION_BOX_CORNER_RADIUS: f32 = 7.0;
const DETECTION_BOX_OPACITY: f32 = 0.85;
const DETECTION_STROKE_WIDTH: f32 = 1.0;
const DETECTION_LABEL_PADDING: Vec2 = vec2(4.0, 2.0);
const DETECTION_LABEL_FONT_SIZE: f32 = 12.0;
const DETECTION_LABEL_BOLD_OFFSET: f32 = 0.6;
const MAXIMUM_INSIDE_LABEL_FRACTION: f32 = 0.5;

#[derive(Clone, Copy)]
enum DetectionLabelCorner {
    TopLeft,
    BottomLeft,
}

#[derive(Clone, Copy)]
enum DetectionLabelPlacement {
    Inside,
    Outside,
    Clamped,
}

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

    pub(super) fn paint(&self, painter: &ImageOverlayPainter, image_time: Time) {
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
    confidence_thresholds: ConfidenceThresholds,
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
        slot.active = overlay_value
            .and_then(|value| value.get("active"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        for definition in T::CONFIDENCE_THRESHOLDS {
            let threshold = slot.confidence_thresholds.get_mut(definition.kind);
            *threshold = overlay_value
                .and_then(|value| value.get(definition.storage_key))
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .unwrap_or(*threshold)
                .clamp(0.0, 1.0);
        }
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
            confidence_thresholds: ConfidenceThresholds::default(),
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
        if self.active && !T::CONFIDENCE_THRESHOLDS.is_empty() {
            ui.indent(T::STORAGE_KEY, |ui| {
                for definition in T::CONFIDENCE_THRESHOLDS {
                    let threshold = self.confidence_thresholds.get_mut(definition.kind);
                    ui.horizontal(|ui| {
                        ui.label(definition.label);
                        ui.add(
                            DragValue::new(threshold)
                                .range(0.0..=1.0)
                                .speed(0.01)
                                .fixed_decimals(2),
                        );
                    });
                }
            });
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

    fn paint(&self, painter: &ImageOverlayPainter, image_time: Time) {
        if let Some(overlay) = &self.overlay {
            overlay.paint(painter, image_time, &self.confidence_thresholds);
        }
    }

    fn latest_time(&self) -> Option<Time> {
        self.overlay.as_ref().and_then(ImageOverlay::latest_time)
    }

    fn save(&self) -> Value {
        let mut value = serde_json::Map::new();
        value.insert("active".to_string(), json!(self.active));
        for definition in T::CONFIDENCE_THRESHOLDS {
            value.insert(
                definition.storage_key.to_string(),
                json!(self.confidence_thresholds.get(definition.kind)),
            );
        }
        Value::Object(value)
    }
}

#[derive(Clone, Copy)]
pub(super) enum ConfidenceThresholdKind {
    BoundingBox,
    Keypoint,
}

pub(super) struct ConfidenceThresholds {
    pub(super) bounding_box: f32,
    pub(super) keypoint: f32,
}

impl Default for ConfidenceThresholds {
    fn default() -> Self {
        Self {
            bounding_box: 0.5,
            keypoint: 0.8,
        }
    }
}

impl ConfidenceThresholds {
    fn get(&self, kind: ConfidenceThresholdKind) -> f32 {
        match kind {
            ConfidenceThresholdKind::BoundingBox => self.bounding_box,
            ConfidenceThresholdKind::Keypoint => self.keypoint,
        }
    }

    fn get_mut(&mut self, kind: ConfidenceThresholdKind) -> &mut f32 {
        match kind {
            ConfidenceThresholdKind::BoundingBox => &mut self.bounding_box,
            ConfidenceThresholdKind::Keypoint => &mut self.keypoint,
        }
    }
}

pub(super) struct ConfidenceThresholdDefinition {
    kind: ConfidenceThresholdKind,
    label: &'static str,
    storage_key: &'static str,
}

impl ConfidenceThresholdDefinition {
    pub(super) const fn new(
        kind: ConfidenceThresholdKind,
        label: &'static str,
        storage_key: &'static str,
    ) -> Self {
        Self {
            kind,
            label,
            storage_key,
        }
    }
}

pub(super) trait ImageOverlay: Sized {
    const NAME: &'static str;
    const STORAGE_KEY: &'static str;
    const CONFIDENCE_THRESHOLDS: &'static [ConfidenceThresholdDefinition] = &[];

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext;

    fn paint(
        &self,
        painter: &ImageOverlayPainter,
        image_time: Time,
        confidence_thresholds: &ConfidenceThresholds,
    );

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

    pub(super) fn detection_line_segment(
        &self,
        start: Point2<Pixel>,
        end: Point2<Pixel>,
        color: Color32,
    ) {
        self.line_segment(start, end, Stroke::new(DETECTION_STROKE_WIDTH, color));
    }

    pub(super) fn detection_box(
        &self,
        bounding_box: BoundingBox,
        class_label: String,
        class_color: Color32,
    ) {
        let rect = Rect::from_min_max(
            self.position(bounding_box.area.min),
            self.position(bounding_box.area.max),
        );
        if !rect.is_positive() {
            return;
        }

        let translucent_color = class_color.gamma_multiply(DETECTION_BOX_OPACITY);
        self.painter.rect_stroke(
            rect,
            CornerRadius::same(DETECTION_BOX_CORNER_RADIUS as u8),
            self.stroke(Stroke::new(DETECTION_STROKE_WIDTH, translucent_color)),
            StrokeKind::Inside,
        );
        let confidence_rect = self.detection_label(
            rect,
            DetectionLabelCorner::TopLeft,
            format!("{:.2}", bounding_box.confidence),
            translucent_color,
            class_color,
            None,
        );
        self.detection_label(
            rect,
            DetectionLabelCorner::BottomLeft,
            class_label,
            translucent_color,
            class_color,
            confidence_rect,
        );
    }

    fn detection_label(
        &self,
        bounding_box_rect: Rect,
        corner: DetectionLabelCorner,
        text: String,
        background_color: Color32,
        class_color: Color32,
        occupied_rect: Option<Rect>,
    ) -> Option<Rect> {
        let image_rect = self.rect.intersect(self.painter.clip_rect());
        if !image_rect.is_positive() || !bounding_box_rect.intersects(image_rect) {
            return None;
        }
        let text_color = contrast_text_color(class_color);
        let galley = self.painter.layout_no_wrap(
            text,
            FontId::proportional(DETECTION_LABEL_FONT_SIZE),
            text_color,
        );
        let label_size =
            galley.size() + 2.0 * DETECTION_LABEL_PADDING + vec2(DETECTION_LABEL_BOLD_OFFSET, 0.0);
        let (label_rect, placement) = detection_label_rect(
            bounding_box_rect,
            image_rect,
            label_size,
            corner,
            occupied_rect,
        );
        if matches!(placement, DetectionLabelPlacement::Outside) {
            let fill_points = outside_bounding_box_corner_fill(bounding_box_rect, corner);
            self.painter.add(Shape::mesh(colored_polygon_mesh(
                &fill_points,
                background_color,
            )));
        }
        self.painter.rect_filled(
            label_rect,
            match placement {
                DetectionLabelPlacement::Inside => detection_label_corner_radius(corner, true),
                DetectionLabelPlacement::Outside => detection_label_corner_radius(corner, false),
                DetectionLabelPlacement::Clamped => {
                    CornerRadius::same(DETECTION_BOX_CORNER_RADIUS as u8)
                }
            },
            background_color,
        );
        let clipped_painter = self.painter.with_clip_rect(label_rect.intersect(self.rect));
        let text_position = label_rect.min + DETECTION_LABEL_PADDING;
        clipped_painter.galley(text_position, galley.clone(), text_color);
        clipped_painter.galley(
            text_position + vec2(DETECTION_LABEL_BOLD_OFFSET, 0.0),
            galley,
            text_color,
        );

        Some(label_rect)
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

fn detection_label_rect(
    bounding_box_rect: Rect,
    image_rect: Rect,
    label_size: Vec2,
    corner: DetectionLabelCorner,
    occupied_rect: Option<Rect>,
) -> (Rect, DetectionLabelPlacement) {
    let inside_min = match corner {
        DetectionLabelCorner::TopLeft => bounding_box_rect.left_top(),
        DetectionLabelCorner::BottomLeft => pos2(
            bounding_box_rect.left(),
            bounding_box_rect.bottom() - label_size.y,
        ),
    };
    let inside_rect = Rect::from_min_size(inside_min, label_size);
    let fits_inside = label_size.x <= bounding_box_rect.width() * MAXIMUM_INSIDE_LABEL_FRACTION
        && label_size.y <= bounding_box_rect.height() * MAXIMUM_INSIDE_LABEL_FRACTION
        && bounding_box_rect.contains_rect(inside_rect)
        && image_rect.contains_rect(inside_rect)
        && occupied_rect.is_none_or(|occupied| !occupied.intersect(inside_rect).is_positive());
    if fits_inside {
        return (inside_rect, DetectionLabelPlacement::Inside);
    }

    let outside_min = match corner {
        DetectionLabelCorner::TopLeft => pos2(
            bounding_box_rect.left(),
            bounding_box_rect.top() - label_size.y,
        ),
        DetectionLabelCorner::BottomLeft => {
            pos2(bounding_box_rect.left(), bounding_box_rect.bottom())
        }
    };
    let outside_rect = Rect::from_min_size(outside_min, label_size);
    if image_rect.contains_rect(outside_rect)
        && occupied_rect.is_none_or(|occupied| !occupied.intersect(outside_rect).is_positive())
    {
        return (outside_rect, DetectionLabelPlacement::Outside);
    }

    // Prefer an inward label at image edges, even if it covers more of the box.
    // A label larger than the visible image is clipped to the available area.
    let size = label_size.min(image_rect.size().max(Vec2::ZERO));
    let min = inside_min.max(image_rect.min).min(image_rect.max - size);
    let mut rect = Rect::from_min_size(min, size);
    if let Some(occupied) = occupied_rect
        && occupied.intersect(rect).is_positive()
    {
        for y in [occupied.bottom(), occupied.top() - size.y] {
            let candidate = Rect::from_min_size(pos2(min.x, y), size);
            if image_rect.contains_rect(candidate) && !occupied.intersect(candidate).is_positive() {
                rect = candidate;
                break;
            }
        }
    }
    (rect, DetectionLabelPlacement::Clamped)
}

fn detection_label_corner_radius(corner: DetectionLabelCorner, is_inside: bool) -> CornerRadius {
    let radius = DETECTION_BOX_CORNER_RADIUS as u8;
    match corner {
        DetectionLabelCorner::TopLeft => CornerRadius {
            nw: radius,
            ne: 0,
            sw: 0,
            se: if is_inside { radius } else { 0 },
        },
        DetectionLabelCorner::BottomLeft => CornerRadius {
            nw: 0,
            ne: if is_inside { radius } else { 0 },
            sw: radius,
            se: 0,
        },
    }
}

fn outside_bounding_box_corner_fill(
    bounding_box_rect: Rect,
    corner: DetectionLabelCorner,
) -> [Pos2; 6] {
    const ARC_SEGMENTS: usize = 4;
    let radius = DETECTION_BOX_CORNER_RADIUS
        .min(bounding_box_rect.width() * 0.5)
        .min(bounding_box_rect.height() * 0.5);
    let (outer_corner, arc_center, start_angle, end_angle) = match corner {
        DetectionLabelCorner::TopLeft => (
            bounding_box_rect.left_top(),
            bounding_box_rect.left_top() + vec2(radius, radius),
            -std::f32::consts::FRAC_PI_2,
            -std::f32::consts::PI,
        ),
        DetectionLabelCorner::BottomLeft => (
            bounding_box_rect.left_bottom(),
            bounding_box_rect.left_bottom() + vec2(radius, -radius),
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ),
    };
    std::array::from_fn(|index| {
        if index == 0 {
            return outer_corner;
        }
        let angle =
            start_angle + (end_angle - start_angle) * (index - 1) as f32 / ARC_SEGMENTS as f32;
        arc_center + vec2(angle.cos() * radius, angle.sin() * radius)
    })
}

fn colored_polygon_mesh(points: &[Pos2], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for &point in points {
        mesh.colored_vertex(point, color);
    }
    for index in 1..points.len().saturating_sub(1) {
        mesh.add_triangle(0, index as u32, index as u32 + 1);
    }
    mesh
}

fn contrast_text_color(background_color: Color32) -> Color32 {
    let [red, green, blue, _] = background_color.to_srgba_unmultiplied();
    let luminance = 0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32;
    if luminance > 150.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}
