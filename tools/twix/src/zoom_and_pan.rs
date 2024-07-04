use coordinate_systems::Screen;
use eframe::egui::{pos2, Response, Ui};
use linear_algebra::{point, IntoTransform, Transform};
use nalgebra::{vector, Similarity2, Translation2};

use crate::twix_painter::TwixPainter;

#[derive(Default)]
pub struct ZoomAndPanTransform {
    transformation: Transform<Screen, Screen, Similarity2<f32>>,
}

impl ZoomAndPanTransform {
    pub fn apply<Frame>(&mut self, ui: &Ui, painter: &mut TwixPainter<Frame>, response: &Response) {
        if response.double_clicked() {
            self.transformation = Similarity2::identity().framed_transform();
        }

        let pointer_position = match ui.input(|input| input.pointer.interact_pos()) {
            Some(position) if response.rect.contains(position) => position,
            _ => {
                painter.append_transform(self.transformation);
                return;
            }
        };

        let zoom_factor = 1.01_f32.powf(ui.input(|input| input.smooth_scroll_delta.y));
        let zoom_transform =
            Similarity2::from_scaling(zoom_factor).framed_transform::<Screen, Screen>();

        let pointer_after_zoom = {
            let pointer = point![pointer_position.x, pointer_position.y];
            let pointer_after_zoom = zoom_transform * pointer;

            pos2(pointer_after_zoom.x(), pointer_after_zoom.y())
        };

        let shift_from_zoom = pointer_position - pointer_after_zoom;

        let pixel_drag = vector![
            response.drag_delta().x,
            painter.orientation.sign() * response.drag_delta().y
        ];
        self.transformation.inner.append_scaling_mut(zoom_factor);
        let zoom_shift = vector![
            shift_from_zoom.x,
            painter.orientation.sign() * shift_from_zoom.y
        ];
        self.transformation
            .inner
            .append_translation_mut(&Translation2::from(pixel_drag + zoom_shift));
        painter.append_transform(self.transformation);
    }
}
