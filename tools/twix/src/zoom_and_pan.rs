use eframe::egui::{Response, Ui};
use linear_algebra::IntoTransform;
use nalgebra::{vector, Similarity2, Translation2};

use crate::twix_painter::TwixPainter;

#[derive(Default)]
pub struct ZoomAndPanTransform {
    pub transformation: Similarity2<f32>,
}

impl ZoomAndPanTransform {
    pub fn apply<Frame>(
        &mut self,
        ui: &mut Ui,
        painter: &mut TwixPainter<Frame>,
        response: &Response,
    ) {
        if response.double_clicked() {
            self.transformation = Similarity2::identity();
        }
        let pointer_position = match ui.input(|input| input.pointer.interact_pos()) {
            Some(position) if response.rect.contains(position) => position,
            _ => return,
        };

        let pointer_in_world_before_zoom = painter.transform_pixel_to_world(pointer_position);
        let zoom_factor = 1.01_f32.powf(ui.input(|input| input.smooth_scroll_delta.y));
        let zoom_transform = Similarity2::from_scaling(zoom_factor);
        painter.append_transform(zoom_transform.framed_transform());
        let pointer_in_pixel_after_zoom =
            painter.transform_world_to_pixel(pointer_in_world_before_zoom);
        let shift_from_zoom = pointer_position - pointer_in_pixel_after_zoom;
        let pixel_drag = vector![response.drag_delta().x, response.drag_delta().y];
        self.transformation.append_scaling_mut(zoom_factor);
        let zoom_shift = vector![shift_from_zoom.x, shift_from_zoom.y];
        self.transformation
            .append_translation_mut(&Translation2::from(pixel_drag + zoom_shift));
    }
}
