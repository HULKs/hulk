use coordinate_systems::Pixel;
use eframe::egui::{
    Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, pos2, vec2,
};
use types::bounding_box::BoundingBox;

use super::TwixPainter;

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

impl TwixPainter<Pixel> {
    pub fn detection_box(
        &self,
        bounding_box: BoundingBox,
        class_label: String,
        class_color: Color32,
    ) {
        let rect = Rect::from_min_max(
            self.transform_world_to_pixel(bounding_box.area.min),
            self.transform_world_to_pixel(bounding_box.area.max),
        );
        if !rect.is_positive() {
            return;
        }

        let translucent_color = class_color.gamma_multiply(DETECTION_BOX_OPACITY);
        self.painter.rect_stroke(
            rect,
            CornerRadius::same(DETECTION_BOX_CORNER_RADIUS as u8),
            self.transform_stroke(Stroke::new(DETECTION_STROKE_WIDTH, translucent_color)),
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
        let image_rect = self.pixel_rect.intersect(self.painter.clip_rect());
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
        let clipped_painter = self.painter.with_clip_rect(label_rect);
        let text_position = label_rect.min + DETECTION_LABEL_PADDING;
        clipped_painter.galley(text_position, galley.clone(), text_color);
        clipped_painter.galley(
            text_position + vec2(DETECTION_LABEL_BOLD_OFFSET, 0.0),
            galley,
            text_color,
        );

        Some(label_rect)
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
