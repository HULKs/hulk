use std::f32::consts::{PI, TAU};

use eframe::{
    egui::{Painter, Response, Sense, Ui},
    emath::{Pos2, Rect},
    epaint::{Color32, PathShape, Rounding, Shape, Stroke},
};
use nalgebra::{point, vector, Isometry2, Point2, Rotation2, Similarity2, Vector2};
use types::{Arc, Circle, FieldDimensions, Orientation, PathSegment};

pub enum CoordinateSystem {
    RightHand,
    LeftHand,
}

impl CoordinateSystem {
    fn y_scale(&self) -> f32 {
        match self {
            CoordinateSystem::RightHand => -1.0,
            CoordinateSystem::LeftHand => 1.0,
        }
    }
}

pub struct TwixPainter {
    painter: Painter,
    pixel_rect: Rect,
    world_to_pixel: Similarity2<f32>,
    camera_coordinate_system: CoordinateSystem,
}

impl TwixPainter {
    pub fn allocate_new(ui: &mut Ui) -> (Response, Self) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());
        let pixel_rect = response.rect;
        let world_to_pixel = Similarity2::new(
            vector![pixel_rect.left_top().x, pixel_rect.left_top().y],
            0.0,
            1.0,
        );
        let twix_painter = Self {
            painter,
            pixel_rect,
            world_to_pixel,
            camera_coordinate_system: CoordinateSystem::RightHand,
        };
        (response, twix_painter)
    }

    pub fn paint_at(ui: &mut Ui, pixel_rect: Rect) -> Self {
        let painter = ui.painter_at(pixel_rect);
        let world_to_pixel = Similarity2::new(
            vector![pixel_rect.left_top().x, pixel_rect.left_top().y],
            0.0,
            1.0,
        );
        Self {
            painter,
            pixel_rect,
            world_to_pixel,
            camera_coordinate_system: CoordinateSystem::RightHand,
        }
    }

    pub fn with_camera(
        self,
        camera_dimensions: Vector2<f32>,
        world_to_camera: Similarity2<f32>,
        camera_coordinate_system: CoordinateSystem,
    ) -> Self {
        let width_scale = self.pixel_rect.width() / camera_dimensions.x;
        let height_scale = self.pixel_rect.height() / camera_dimensions.y;
        let top_left = vector![self.pixel_rect.left_top().x, self.pixel_rect.left_top().y];
        let camera_to_pixel = Similarity2::new(top_left, 0.0, width_scale.min(height_scale));
        Self {
            painter: self.painter,
            pixel_rect: self.pixel_rect,
            world_to_pixel: camera_to_pixel * world_to_camera,
            camera_coordinate_system,
        }
    }

    pub fn with_map_transforms(self, field_dimensions: &FieldDimensions) -> Self {
        let length = field_dimensions.length + field_dimensions.border_strip_width * 2.0;
        let width = field_dimensions.width + field_dimensions.border_strip_width * 2.0;
        let dimensions = vector![length, width];
        let world_to_camera = Similarity2::new(vector![length / 2.0, width / 2.0], 0.0, 1.0);
        self.with_camera(dimensions, world_to_camera, CoordinateSystem::RightHand)
    }

    pub fn append_transform(&mut self, transformation: Similarity2<f32>) {
        self.world_to_pixel = transformation * self.world_to_pixel;
    }

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

    pub fn ball(&self, position: Point2<f32>, radius: f32) {
        self.circle(
            position,
            radius,
            Color32::WHITE,
            Stroke {
                width: radius / 8.0,
                color: Color32::BLACK,
            },
        );

        (0..5).for_each(|index| {
            let angle = index as f32 * PI * 2.0 / 5.0;
            let position = position + vector![angle.cos(), angle.sin()] * radius * 0.7;
            self.n_gon(5, position, radius / 3.0, Color32::BLACK);
        });
        self.n_gon(5, position, radius / 3.0, Color32::BLACK);
    }

    pub fn n_gon(&self, corners: usize, position: Point2<f32>, radius: f32, fill_color: Color32) {
        let points: Vec<_> = (0..corners)
            .map(|index| {
                self.transform_world_to_pixel({
                    let angle = index as f32 * PI * 2.0 / corners as f32;
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

    pub fn path(
        &self,
        robot_to_field: Isometry2<f32>,
        path: Vec<PathSegment>,
        line_color: Color32,
        arc_color: Color32,
        width: f32,
    ) {
        for segment in path {
            match segment {
                PathSegment::LineSegment(line_segment) => self.line_segment(
                    robot_to_field * line_segment.0,
                    robot_to_field * line_segment.1,
                    Stroke {
                        width,
                        color: line_color,
                    },
                ),
                PathSegment::Arc(arc, orientation) => self.arc(
                    arc,
                    orientation,
                    Stroke {
                        width,
                        color: arc_color,
                    },
                    robot_to_field,
                ),
            }
        }
    }

    pub fn pose(
        &self,
        pose: Isometry2<f32>,
        circle_radius: f32,
        line_length: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) {
        self.circle(pose * Point2::origin(), circle_radius, fill_color, stroke);
        self.line_segment(
            pose * Point2::origin(),
            pose * point![line_length, 0.0],
            stroke,
        );
    }

    pub fn transform_world_to_pixel(&self, point: Point2<f32>) -> Pos2 {
        let normalized = self.world_to_pixel
            * point![point.x, point.y * self.camera_coordinate_system.y_scale()];
        Pos2 {
            x: normalized.x,
            y: normalized.y,
        }
    }

    pub fn transform_pixel_to_world(&self, pos: Pos2) -> Point2<f32> {
        let world_point = self.world_to_pixel.inverse() * point![pos.x, pos.y];
        point![
            world_point.x,
            world_point.y * self.camera_coordinate_system.y_scale()
        ]
    }

    fn transform_stroke(&self, stroke: Stroke) -> Stroke {
        Stroke {
            width: stroke.width * self.world_to_pixel.scaling(),
            ..stroke
        }
    }

    pub fn line_segment(&self, start: Point2<f32>, end: Point2<f32>, stroke: Stroke) {
        let start = self.transform_world_to_pixel(start);
        let end = self.transform_world_to_pixel(end);
        let stroke = self.transform_stroke(stroke);
        self.painter.line_segment([start, end], stroke);
    }

    pub fn rect_filled(&self, min: Point2<f32>, max: Point2<f32>, fill_color: Color32) {
        let rect = Rect {
            min: self.transform_world_to_pixel(min),
            max: self.transform_world_to_pixel(max),
        };
        self.painter
            .rect_filled(sort_rect(rect), Rounding::none(), fill_color);
    }

    pub fn rect_stroke(&self, min: Point2<f32>, max: Point2<f32>, stroke: Stroke) {
        let rect = Rect {
            min: self.transform_world_to_pixel(min),
            max: self.transform_world_to_pixel(max),
        };
        let stroke = self.transform_stroke(stroke);
        self.painter
            .rect_stroke(sort_rect(rect), Rounding::none(), stroke);
    }

    pub fn circle(&self, center: Point2<f32>, radius: f32, fill_color: Color32, stroke: Stroke) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.world_to_pixel.scaling();
        let stroke = self.transform_stroke(stroke);
        self.painter.circle(center, radius, fill_color, stroke);
    }

    pub fn circle_filled(&self, center: Point2<f32>, radius: f32, fill_color: Color32) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.world_to_pixel.scaling();
        self.painter.circle_filled(center, radius, fill_color);
    }

    pub fn circle_stroke(&self, center: Point2<f32>, radius: f32, stroke: Stroke) {
        let center = self.transform_world_to_pixel(center);
        let radius = radius * self.world_to_pixel.scaling();
        let stroke = self.transform_stroke(stroke);
        self.painter.circle_stroke(center, radius, stroke);
    }

    pub fn arc(&self, arc: Arc, orientation: Orientation, stroke: Stroke, pose: Isometry2<f32>) {
        let Arc {
            circle: Circle { center, radius },
            start,
            end,
        } = arc;
        let start_relative = start - center;
        let end_relative = end - center;
        let angle_difference = start_relative.angle(&end_relative);
        let end_right_of_start = Orientation::Counterclockwise
            .rotate_vector_90_degrees(start_relative)
            .dot(&end_relative)
            < 0.0;
        let counterclockwise_angle_difference = if end_right_of_start {
            2.0 * PI - angle_difference
        } else {
            angle_difference
        };

        let signed_angle_difference = match orientation {
            Orientation::Clockwise => -2.0 * PI + counterclockwise_angle_difference,
            Orientation::Counterclockwise => counterclockwise_angle_difference,
            Orientation::Colinear => 0.0,
        };

        const PIXELS_PER_SAMPLE: f32 = 5.0;
        let samples = 1.max(
            (signed_angle_difference.abs() * radius * self.world_to_pixel.scaling()
                / PIXELS_PER_SAMPLE) as usize,
        );
        let points = (0..samples + 1)
            .map(|index| {
                let angle = signed_angle_difference / samples as f32 * index as f32;
                let point = pose * (center + Rotation2::new(angle) * start_relative);
                self.transform_world_to_pixel(point)
            })
            .collect();

        let stroke = self.transform_stroke(stroke);

        self.painter
            .add(Shape::Path(PathShape::line(points, stroke)));
    }

    #[allow(unused)]
    pub fn ellipse(&self, position: Vector2<f32>, w: f32, h: f32, theta: f32, stroke: Stroke) {
        let samples = 360;
        let points = (0..samples + 1)
            .map(|i| {
                let t = i as f32 * TAU / samples as f32;
                let x = w * theta.cos() * t.cos() - h * theta.sin() * t.sin();
                let y = w * theta.sin() * t.cos() + h * theta.cos() * t.sin();
                self.transform_world_to_pixel(point![x, y] + position)
            })
            .collect();
        let stroke = self.transform_stroke(stroke);
        self.painter
            .add(Shape::Path(PathShape::line(points, stroke)));
    }
}

fn sort_rect(rect: Rect) -> Rect {
    let Rect { min, max } = rect;
    Rect {
        min: Pos2 {
            x: min.x.min(max.x),
            y: min.y.min(max.y),
        },
        max: Pos2 {
            x: min.x.max(max.x),
            y: min.y.max(max.y),
        },
    }
}
