use coordinate_systems::Screen;
use eframe::egui::{PointerButton, Response, Ui, pos2};
use linear_algebra::{IntoTransform, Transform, point};
use nalgebra::{Similarity2, Translation2, vector};
use serde::{Deserialize, Serialize};

use crate::twix_painter::TwixPainter;

#[derive(Default, Serialize, Deserialize)]
pub struct ZoomAndPanTransform {
    pub transformation: Transform<Screen, Screen, Similarity2<f32>>,
}

impl ZoomAndPanTransform {
    pub fn apply_transform<Frame>(&self, painter: &mut TwixPainter<Frame>) {
        painter.append_transform(self.transformation);
    }

    pub fn process_input<Frame>(
        &mut self,
        ui: &Ui,
        painter: &mut TwixPainter<Frame>,
        response: &Response,
        reset_transform: Option<Transform<Screen, Screen, Similarity2<f32>>>,
    ) {
        if response.double_clicked_by(PointerButton::Primary)
            || response.double_clicked_by(PointerButton::Secondary)
        {
            self.transformation =
                reset_transform.unwrap_or_else(|| Similarity2::identity().framed_transform());
        }

        let pointer_position = match ui.input(|input| input.pointer.interact_pos()) {
            Some(position) if response.rect.contains(position) => position,
            _ => return,
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
    }

    pub fn apply<Frame>(&mut self, ui: &Ui, painter: &mut TwixPainter<Frame>, response: &Response) {
        self.process_input(ui, painter, response, None);
        self.apply_transform(painter);
    }
}
