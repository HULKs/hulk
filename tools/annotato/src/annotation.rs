use eframe::epaint::Pos2;
use egui_plot::PlotPoint;
use serde::{Deserialize, Serialize};

use crate::{boundingbox::BoundingBox, classes::Classes};

#[derive(Serialize, Deserialize, Debug)]
pub struct AnnotationFormat {
    pub points: [[f32; 2]; 2],
    pub class: Classes,
}

impl From<AnnotationFormat> for BoundingBox {
    fn from(value: AnnotationFormat) -> Self {
        let [[min_x, min_y], [max_x, max_y]] = value.points;
        let class = value.class;

        Self {
            corner: PlotPoint::new(min_x as f64, 480. - max_y as f64),
            opposing_corner: PlotPoint::new(max_x as f64, 480. - min_y as f64),
            class,
        }
    }
}

impl From<BoundingBox> for AnnotationFormat {
    fn from(value: BoundingBox) -> Self {
        let rect = value.rect();
        let Pos2 { x: x1, y: y1 } = rect.left_top();
        let Pos2 { x: x2, y: y2 } = rect.right_bottom();

        Self {
            points: [[x1, 480. - y2], [x2, 480. - y1]],
            class: value.class,
        }
    }
}
