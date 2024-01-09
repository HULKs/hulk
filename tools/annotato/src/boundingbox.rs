use eframe::epaint::{Pos2, Rect};
use egui_plot::{PlotPoint, PlotPoints, Polygon};

use crate::classes::Class;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub corner: PlotPoint,
    pub opposing_corner: PlotPoint,
    pub class: Class,
}

impl From<&BoundingBox> for Polygon {
    fn from(value: &BoundingBox) -> Self {
        let x1 = value.corner.x;
        let y1 = value.corner.y;
        let x2 = value.opposing_corner.x;
        let y2 = value.opposing_corner.y;
        let plotpoints = PlotPoints::new(vec![[x1, y1], [x1, y2], [x2, y2], [x2, y1]]);
        Polygon::new(plotpoints)
    }
}

impl BoundingBox {
    pub fn new(corner: PlotPoint, opposing_corner: PlotPoint, class: Class) -> Self {
        BoundingBox {
            corner,
            opposing_corner,
            class,
        }
    }

    pub fn set_opposing_corner(&mut self, plot_bottom_right: PlotPoint) {
        self.opposing_corner = plot_bottom_right;
    }

    pub fn rect(&self) -> Rect {
        let to_pos2 = |point: PlotPoint| Pos2::new(point.x as f32, point.y as f32);
        Rect::from_points(&[to_pos2(self.corner), to_pos2(self.opposing_corner)])
    }

    pub fn top_left(&self) -> PlotPoint {
        let x1 = self.corner.x;
        let y1 = self.corner.y;
        let x2 = self.opposing_corner.x;
        let y2 = self.opposing_corner.y;

        PlotPoint::new(x1.min(x2), y1.max(y2))
    }

    pub fn contains(&self, mouse_position: PlotPoint) -> bool {
        let x1 = self.corner.x;
        let y1 = self.corner.y;
        let x2 = self.opposing_corner.x;
        let y2 = self.opposing_corner.y;

        (x1.min(x2)..=x1.max(x2)).contains(&mouse_position.x)
            && (y1.min(y2)..=y1.max(y2)).contains(&mouse_position.y)
    }

    pub fn clip_to_image(&mut self) {
        let x1 = self.corner.x;
        let y1 = self.corner.y;
        let x2 = self.opposing_corner.x;
        let y2 = self.opposing_corner.y;

        self.corner = PlotPoint::new(x1.clamp(0., 640.), y1.clamp(0., 480.));
        self.opposing_corner = PlotPoint::new(x2.clamp(0., 640.), y2.clamp(0., 480.));
    }

    pub fn is_valid(&self) -> bool {
        let rect = self.rect();
        rect.area() >= 4.0
    }

    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let this_rect = self.rect();
        let other_rect = other.rect();

        let intersection = this_rect.intersect(other_rect).area();
        let union = this_rect.area() + other_rect.area() - intersection;

        intersection / union
    }

    fn closest_and_furthest_corner(&self, position: PlotPoint) -> (PlotPoint, PlotPoint) {
        let rect = self.rect();
        let position = Pos2::new(position.x as f32, position.y as f32);

        let corners = [
            rect.left_bottom(),
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
        ];
        let closest = corners
            .iter()
            .min_by(|corner1, corner2| {
                corner1
                    .distance_sq(position)
                    .total_cmp(&corner2.distance_sq(position))
            })
            .unwrap();
        let furthest = corners
            .iter()
            .max_by(|corner1, corner2| {
                corner1
                    .distance_sq(position)
                    .total_cmp(&corner2.distance_sq(position))
            })
            .unwrap();

        (
            PlotPoint::new(closest.x as f64, closest.y as f64),
            PlotPoint::new(furthest.x as f64, furthest.y as f64),
        )
    }

    pub fn get_closest_corner(&self, position: PlotPoint) -> PlotPoint {
        self.closest_and_furthest_corner(position).0
    }

    pub fn closest_corner_distance_sq(&self, position: PlotPoint) -> f32 {
        let closest = self.get_closest_corner(position);
        closest.to_pos2().distance_sq(position.to_pos2())
    }

    pub fn has_corner_at(&self, position: PlotPoint) -> bool {
        let closest = self.get_closest_corner(position);
        let closest = Pos2::new(closest.x as f32, closest.y as f32);
        let position = Pos2::new(position.x as f32, position.y as f32);

        closest.distance(position) < 5.0
    }

    pub fn prepare_for_corner_move(&mut self, position: PlotPoint) {
        let (closest, furthest) = self.closest_and_furthest_corner(position);
        self.corner = furthest;
        self.opposing_corner = closest;
    }

    pub fn to_annotation(&self) -> (Class, [f32; 4]) {
        let rect = self.rect();
        let Pos2 { x: min_x, y: min_y } = rect.left_top();
        let Pos2 { x: max_x, y: max_y } = rect.right_bottom();

        (
            self.class,
            [
                min_x / 640.,
                (480. - max_y) / 480.,
                max_x / 640.,
                (480. - min_y) / 480.,
            ],
        )
    }

    pub fn from_annotation((class, [min_x, min_y, max_x, max_y]): (Class, [f32; 4])) -> Self {
        let corner = PlotPoint::new(min_x * 640., (1.0 - max_y) * 480.);
        let opposing_corner = PlotPoint::new(max_x * 640., (1. - min_y) * 480.);

        Self::new(corner, opposing_corner, class)
    }
}
