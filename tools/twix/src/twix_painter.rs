use std::{
    f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, TAU},
    marker::PhantomData,
};

use eframe::{
    egui::{pos2, Context, Painter, Response, Sense, TextureId, Ui},
    emath::{Pos2, Rect},
    epaint::{Color32, PathShape, Shape, Stroke},
};
use nalgebra::{SMatrix, Similarity2};

use coordinate_systems::{Field, Ground, Screen};
use geometry::{arc::Arc, circle::Circle, rectangle::Rectangle};
use linear_algebra::{point, vector, IntoTransform, Isometry2, Point2, Pose2, Transform, Vector2};
use types::{field_dimensions::FieldDimensions, planned_path::PathSegment};

type ScreenTransform<Frame> = Transform<Frame, Screen, Similarity2<f32>>;

pub struct TwixPainter<World> {
    painter: Painter,
    pixel_rect: Rect,
    pub orientation: Orientation,
    world_to_pixel: ScreenTransform<World>,
    frame: PhantomData<World>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Orientation {
    #[default]
    LeftHanded,
    RightHanded,
}

impl Orientation {
    pub fn sign(self) -> f32 {
        match self {
            Orientation::LeftHanded => 1.0,
            Orientation::RightHanded => -1.0,
        }
    }
}

impl<World> TwixPainter<World> {
    pub fn allocate(
        ui: &mut Ui,
        dimension: Vector2<World>,
        origin: Point2<World>,
        orientation: Orientation,
    ) -> (Response, Self) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

        let pixel_rect = response.rect;

        let screen_width = pixel_rect.width();
        let x_scale = screen_width / dimension.x();

        let screen_height = pixel_rect.height();
        let y_scale = screen_height / dimension.y();

        let world_to_camera = Similarity2::new(origin.inner.coords, 0.0, 1.0);

        let camera_to_pixel = Similarity2::new(
            nalgebra::vector![
                pixel_rect.left_top().x,
                orientation.sign() * pixel_rect.left_top().y
            ],
            0.0,
            x_scale.min(y_scale),
        );

        (
            response,
            Self {
                painter,
                pixel_rect,
                orientation,
                world_to_pixel: (camera_to_pixel * world_to_camera).framed_transform(),
                frame: PhantomData,
            },
        )
    }

    pub fn transform_painter<NewFrame>(
        &self,
        isometry: Isometry2<World, NewFrame>,
    ) -> TwixPainter<NewFrame> {
        TwixPainter::<NewFrame> {
            painter: self.painter.clone(),
            pixel_rect: self.pixel_rect,
            orientation: self.orientation,
            world_to_pixel: (self.world_to_pixel * isometry.inverse()),
            frame: PhantomData,
        }
    }

    pub fn paint_at(ui: &mut Ui, pixel_rect: Rect) -> Self {
        let painter = ui.painter_at(pixel_rect);
        let world_to_pixel = Similarity2::new(
            nalgebra::vector![pixel_rect.left_top().x, -pixel_rect.left_top().y],
            0.0,
            1.0,
        );
        let world_to_pixel = world_to_pixel.framed_transform();
        Self {
            painter,
            pixel_rect,
            world_to_pixel,
            orientation: Orientation::default(),
            frame: PhantomData,
        }
    }

    pub fn with_camera(
        self,
        camera_dimensions: Vector2<World, f32>,
        world_to_camera: Similarity2<f32>,
        orientation: Orientation,
    ) -> Self {
        let width_scale = self.pixel_rect.width() / camera_dimensions.x();
        let height_scale = self.pixel_rect.height() / camera_dimensions.y();
        let top_left =
            nalgebra::vector![self.pixel_rect.left_top().x, self.pixel_rect.left_top().y,];
        let camera_to_pixel = Similarity2::new(top_left, 0.0, width_scale.min(height_scale));
        let world_to_pixel = camera_to_pixel * world_to_camera;
        let world_to_pixel = world_to_pixel.framed_transform();
        Self {
            painter: self.painter,
            pixel_rect: self.pixel_rect,
            orientation,
            world_to_pixel,
            frame: PhantomData,
        }
    }

    pub fn append_transform(
        &mut self,
        transformation: Transform<Screen, Screen, Similarity2<f32>>,
    ) {
        self.world_to_pixel = transformation * self.world_to_pixel;
    }

    pub fn context(&self) -> &Context {
        self.painter.ctx()
    }

    pub fn scaling(&self) -> f32 {
        self.world_to_pixel.inner.scaling()
    }

    pub fn arc(&self, arc: Arc<World>, stroke: Stroke) {
        let Arc {
            circle: Circle { center, radius },
            start,
            end,
            direction,
        } = arc;

        let signed_angle_difference = start.angle_to(end, direction);

        const PIXELS_PER_SAMPLE: f32 = 5.0;
        let samples = 1.max(
            (signed_angle_difference.0.abs() * radius * self.scaling() / PIXELS_PER_SAMPLE)
                as usize,
        );
        let points = (0..=samples)
            .map(|index| {
                let angle = signed_angle_difference / samples as f32 * index as f32;
                let point = center + angle.as_direction() * radius;
                self.transform_world_to_pixel(point)
            })
            .collect();

        let stroke = self.transform_stroke(stroke);

        self.painter
            .add(Shape::Path(PathShape::line(points, stroke)));
    }

    pub fn ball(&self, position: Point2<World>, radius: f32, color: Color32) {
        self.circle(
            position,
            radius,
            color,
            Stroke {
                width: radius / 8.0,
                color: Color32::BLACK,
            },
        );

        (0..5).for_each(|index| {
            let angle = index as f32 * TAU / 5.0;
            let position = position + vector![angle.cos(), angle.sin()] * radius * 0.7;
            self.n_gon(5, position, radius / 3.0, Color32::BLACK);
        });
        self.n_gon(5, position, radius / 3.0, Color32::BLACK);
    }

    pub fn n_gon(&self, corners: usize, position: Point2<World>, radius: f32, fill_color: Color32) {
        let points: Vec<_> = (0..corners)
            .map(|index| {
                self.transform_world_to_pixel({
                    let angle = index as f32 * TAU / corners as f32;
                    position + vector![angle.cos(), angle.sin()] * radius
                })
            })
            .collect();
        self.painter.add(Shape::Path(PathShape::convex_polygon(
            points,
            fill_color,
            Stroke::default(),
        )));
    }

    pub fn polygon(&self, points: impl IntoIterator<Item = Point2<World>>, stroke: Stroke) {
        let points: Vec<_> = points
            .into_iter()
            .map(|point| self.transform_world_to_pixel(point))
            .collect();
        let stroke = self.transform_stroke(stroke);
        self.painter
            .add(Shape::Path(PathShape::line(points, stroke)));
    }

    pub fn pose(
        &self,
        pose: Pose2<World>,
        circle_radius: f32,
        line_length: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) {
        let center = pose.position();
        self.circle(center, circle_radius, fill_color, stroke);
        self.line_segment(
            center,
            pose.as_transform::<Ground>() * point![line_length, 0.0],
            stroke,
        );
    }

    pub fn transform_world_to_pixel(&self, point: Point2<World>) -> Pos2 {
        let normalized = self.world_to_pixel * point;
        Pos2 {
            x: normalized.x(),
            y: self.orientation.sign() * normalized.y(),
        }
    }

    pub fn transform_pixel_to_world(&self, pos: Pos2) -> Point2<World> {
        let inverse = self
            .world_to_pixel
            .inner
            .inverse()
            .framed_transform::<Screen, World>();

        inverse * point![pos.x, self.orientation.sign() * pos.y]
    }

    fn transform_stroke(&self, stroke: Stroke) -> Stroke {
        Stroke {
            width: stroke.width * self.scaling(),
            ..stroke
        }
    }

    pub fn line_segment(&self, start: Point2<World>, end: Point2<World>, stroke: Stroke) {
        let start = self.transform_world_to_pixel(start);
        let end = self.transform_world_to_pixel(end);
        let stroke = self.transform_stroke(stroke);
        self.painter.line_segment([start, end], stroke);
    }

    pub fn rect_filled(&self, min: Point2<World>, max: Point2<World>, fill_color: Color32) {
        let right_bottom = point![max.x(), min.y()];
        let left_top = point![min.x(), max.y()];

        let points: Vec<_> = vec![
            self.transform_world_to_pixel(min),
            self.transform_world_to_pixel(right_bottom),
            self.transform_world_to_pixel(max),
            self.transform_world_to_pixel(left_top),
        ];

        self.painter.add(Shape::Path(PathShape::convex_polygon(
            points,
            fill_color,
            Stroke::default(),
        )));
    }

    pub fn rect_stroke(&self, min: Point2<World>, max: Point2<World>, stroke: Stroke) {
        let right_bottom = point![max.x(), min.y()];
        let left_top = point![min.x(), max.y()];

        let points: Vec<_> = vec![
            self.transform_world_to_pixel(min),
            self.transform_world_to_pixel(right_bottom),
            self.transform_world_to_pixel(max),
            self.transform_world_to_pixel(left_top),
        ];

        self.painter.add(Shape::Path(PathShape::convex_polygon(
            points,
            Color32::TRANSPARENT,
            self.transform_stroke(stroke),
        )));
    }

    pub fn circle(&self, center: Point2<World>, radius: f32, fill_color: Color32, stroke: Stroke) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.scaling();
        let stroke = self.transform_stroke(stroke);
        self.painter.circle(center, radius, fill_color, stroke);
    }

    pub fn circle_filled(&self, center: Point2<World>, radius: f32, fill_color: Color32) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.scaling();
        self.painter.circle_filled(center, radius, fill_color);
    }

    pub fn circle_stroke(&self, center: Point2<World>, radius: f32, stroke: Stroke) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.scaling();
        let stroke = self.transform_stroke(stroke);
        self.painter.circle_stroke(center, radius, stroke);
    }

    pub fn ellipse(
        &self,
        position: Point2<World>,
        w: f32,
        h: f32,
        theta: f32,
        stroke: Stroke,
        fill_color: Color32,
    ) {
        let samples = 360;
        let points = (0..samples)
            .map(|i| {
                let t = i as f32 * TAU / samples as f32;
                let x = w * theta.cos() * t.cos() - h * theta.sin() * t.sin();
                let y = w * theta.sin() * t.cos() + h * theta.cos() * t.sin();
                self.transform_world_to_pixel(position + vector![x, y])
            })
            .collect();
        let stroke = self.transform_stroke(stroke);
        self.painter.add(Shape::Path(PathShape::convex_polygon(
            points, fill_color, stroke,
        )));
    }

    pub fn covariance(
        &self,
        position: Point2<World>,
        covariance: SMatrix<f32, 2, 2>,
        stroke: Stroke,
        fill_color: Color32,
    ) {
        let a = covariance.m11;
        let b = covariance.m12;
        let c = covariance.m22;
        let l1 = (a + c) / 2.0 + (((a - c) / 2.0).powi(2) + b.powi(2)).sqrt();
        let l2 = (a + c) / 2.0 - (((a - c) / 2.0).powi(2) + b.powi(2)).sqrt();
        let theta = if b == 0.0 && a >= c {
            0.0
        } else if b == 0.0 && a < c {
            FRAC_PI_2
        } else {
            (l1 - a).atan2(b)
        };
        self.ellipse(position, l1.sqrt(), l2.sqrt(), theta, stroke, fill_color)
    }

    pub fn target(
        &self,
        position: Point2<World>,
        radius: f32,
        stroke: Stroke,
        fill_color: Color32,
    ) {
        self.circle_filled(position, radius, fill_color);
        self.circle_stroke(position, radius, stroke);
        self.line_segment(
            point![
                position.x() - FRAC_1_SQRT_2 * radius,
                position.y() + FRAC_1_SQRT_2 * radius
            ],
            point![
                position.x() + FRAC_1_SQRT_2 * radius,
                position.y() - FRAC_1_SQRT_2 * radius
            ],
            stroke,
        );
        self.line_segment(
            point![
                position.x() + FRAC_1_SQRT_2 * radius,
                position.y() + FRAC_1_SQRT_2 * radius
            ],
            point![
                position.x() - FRAC_1_SQRT_2 * radius,
                position.y() - FRAC_1_SQRT_2 * radius
            ],
            stroke,
        );
    }

    pub fn floating_text(
        &self,
        position: Point2<World>,
        align: eframe::emath::Align2,
        text: String,
        font_id: eframe::epaint::FontId,
        color: Color32,
    ) {
        let position = self.transform_world_to_pixel(position);
        self.painter.text(position, align, text, font_id, color);
    }

    pub fn image(&self, texture_id: TextureId, rect: Rectangle<World>) {
        let Rectangle { min, max } = rect;
        let min = self.transform_world_to_pixel(min);
        let max = self.transform_world_to_pixel(max);
        self.painter.image(
            texture_id,
            Rect::from_two_pos(min, max),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}
impl TwixPainter<Ground> {
    pub fn path(
        &self,
        path: Vec<PathSegment>,
        line_color: Color32,
        arc_color: Color32,
        width: f32,
    ) {
        for segment in path {
            match segment {
                PathSegment::LineSegment(line_segment) => self.line_segment(
                    line_segment.0,
                    line_segment.1,
                    Stroke {
                        width,
                        color: line_color,
                    },
                ),
                PathSegment::Arc(arc) => self.arc(
                    arc,
                    Stroke {
                        width,
                        color: arc_color,
                    },
                ),
            }
        }
    }
}

impl TwixPainter<Field> {
    pub fn field(&self, field_dimensions: &FieldDimensions) {
        let line_stroke = Stroke::new(field_dimensions.line_width, Color32::WHITE);
        let goal_post_stroke =
            Stroke::new(field_dimensions.goal_post_diameter / 8.0, Color32::BLACK);

        // Background
        self.rect_filled(
            point![
                -field_dimensions.length / 2.0 - field_dimensions.border_strip_width,
                -field_dimensions.width / 2.0 - field_dimensions.border_strip_width
            ],
            point![
                field_dimensions.length / 2.0 + field_dimensions.border_strip_width,
                field_dimensions.width / 2.0 + field_dimensions.border_strip_width
            ],
            Color32::DARK_GREEN,
        );

        // Outer lines
        self.rect_stroke(
            point![
                -field_dimensions.length / 2.0,
                -field_dimensions.width / 2.0
            ],
            point![field_dimensions.length / 2.0, field_dimensions.width / 2.0],
            line_stroke,
        );

        // Center line
        self.line_segment(
            point![0.0, -field_dimensions.width / 2.0],
            point![0.0, field_dimensions.width / 2.0],
            line_stroke,
        );

        // Center center
        self.circle_stroke(
            point![0.0, 0.0],
            field_dimensions.center_circle_diameter / 2.0,
            line_stroke,
        );

        // Penalty areas
        self.rect_stroke(
            point![
                -field_dimensions.length / 2.0,
                -field_dimensions.penalty_area_width / 2.0
            ],
            point![
                -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                field_dimensions.penalty_area_width / 2.0
            ],
            line_stroke,
        );
        self.rect_stroke(
            point![
                field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
                -field_dimensions.penalty_area_width / 2.0
            ],
            point![
                field_dimensions.length / 2.0,
                field_dimensions.penalty_area_width / 2.0
            ],
            line_stroke,
        );

        // Goal areas
        self.rect_stroke(
            point![
                -field_dimensions.length / 2.0,
                -field_dimensions.goal_box_area_width / 2.0
            ],
            point![
                -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                field_dimensions.goal_box_area_width / 2.0
            ],
            line_stroke,
        );
        self.rect_stroke(
            point![
                field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length,
                -field_dimensions.goal_box_area_width / 2.0
            ],
            point![
                field_dimensions.length / 2.0,
                field_dimensions.goal_box_area_width / 2.0
            ],
            line_stroke,
        );

        // Penalty spots
        self.line_segment(
            point![
                -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance
                    - field_dimensions.penalty_marker_size / 2.0,
                0.0
            ],
            point![
                -field_dimensions.length / 2.0
                    + field_dimensions.penalty_marker_distance
                    + field_dimensions.penalty_marker_size / 2.0,
                0.0
            ],
            line_stroke,
        );
        self.line_segment(
            point![
                -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance,
                -field_dimensions.penalty_marker_size / 2.0
            ],
            point![
                -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance,
                field_dimensions.penalty_marker_size / 2.0
            ],
            line_stroke,
        );
        self.line_segment(
            point![
                field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance
                    + field_dimensions.penalty_marker_size / 2.0,
                0.0
            ],
            point![
                field_dimensions.length / 2.0
                    - field_dimensions.penalty_marker_distance
                    - field_dimensions.penalty_marker_size / 2.0,
                0.0
            ],
            line_stroke,
        );
        self.line_segment(
            point![
                field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance,
                -field_dimensions.penalty_marker_size / 2.0
            ],
            point![
                field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance,
                field_dimensions.penalty_marker_size / 2.0
            ],
            line_stroke,
        );

        // Goal posts
        self.circle(
            point![
                -field_dimensions.length / 2.0 - field_dimensions.line_width / 2.0,
                -field_dimensions.goal_inner_width / 2.0
                    - field_dimensions.goal_post_diameter / 2.0
            ],
            field_dimensions.goal_post_diameter / 2.0,
            Color32::WHITE,
            goal_post_stroke,
        );
        self.circle(
            point![
                -field_dimensions.length / 2.0 - field_dimensions.line_width / 2.0,
                field_dimensions.goal_inner_width / 2.0 + field_dimensions.goal_post_diameter / 2.0
            ],
            field_dimensions.goal_post_diameter / 2.0,
            Color32::WHITE,
            goal_post_stroke,
        );
        self.circle(
            point![
                field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0,
                -field_dimensions.goal_inner_width / 2.0
                    - field_dimensions.goal_post_diameter / 2.0
            ],
            field_dimensions.goal_post_diameter / 2.0,
            Color32::WHITE,
            goal_post_stroke,
        );
        self.circle(
            point![
                field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0,
                field_dimensions.goal_inner_width / 2.0 + field_dimensions.goal_post_diameter / 2.0
            ],
            field_dimensions.goal_post_diameter / 2.0,
            Color32::WHITE,
            goal_post_stroke,
        );
    }
}
