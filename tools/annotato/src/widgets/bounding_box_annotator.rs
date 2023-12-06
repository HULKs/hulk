use eframe::{
    egui::{Id, Key, PointerButton, Response, RichText, Ui, Widget},
    emath::Align2,
    epaint::{Color32, Stroke, TextureHandle, Vec2},
};
use egui_plot::{Plot, PlotImage, PlotPoint, PlotResponse, Polygon, Text};
use std::hash::Hash;

use crate::{boundingbox::BoundingBox, classes::Classes};

pub struct BoundingBoxAnnotator<'a> {
    id: Id,
    texture_handle: TextureHandle,
    selected_class: Classes,
    bounding_boxes: &'a mut Vec<BoundingBox>,
    box_in_editing: &'a mut Option<BoundingBox>,
}

impl<'a> BoundingBoxAnnotator<'a> {
    pub fn new(
        id_source: impl Hash,
        image: TextureHandle,
        bounding_boxes: &'a mut Vec<BoundingBox>,
        box_in_editing: &'a mut Option<BoundingBox>,
        selected_class: Classes,
    ) -> Self {
        Self {
            id: Id::new(id_source),
            texture_handle: image,
            bounding_boxes,
            box_in_editing,
            selected_class,
        }
    }

    fn delete_box_from(mouse_position: PlotPoint, bbox_list: &mut Vec<BoundingBox>) -> bool {
        if let Some(clicked_bbox_index) = bbox_list
            .iter()
            .enumerate()
            .filter(|(_, bbox)| bbox.contains(mouse_position))
            .min_by(|(_, bbox1), (_, bbox2)| bbox1.rect().area().total_cmp(&bbox2.rect().area()))
            .map(|(idx, _)| idx)
        {
            bbox_list.remove(clicked_bbox_index);
            return true;
        }
        false
    }

    fn handle_bounding_box_input(&mut self, response: &PlotResponse<()>, ui: &Ui) {
        let mouse_position = response
            .response
            .hover_pos()
            .map(|position| response.transform.value_from_position(position));

        let editing_bounding_box = match (
            self.box_in_editing.take(),
            ui.input(|i| i.key_pressed(Key::B))
                || response.response.clicked_by(PointerButton::Primary),
            ui.input(|i| i.key_pressed(Key::G)),
            response.response.clicked_by(PointerButton::Secondary),
        ) {
            (Some(_), _, _, true) => {
                // delete the currently edited bounding box
                None
            }
            (None, _, _, true) => {
                // delete a bounding box
                mouse_position.map(|position| Self::delete_box_from(position, self.bounding_boxes));
                None
            }
            (Some(mut bounding_box), b_pressed, g_pressed, false) if b_pressed || g_pressed => {
                // finish the box
                bounding_box.clip_to_image();
                if bounding_box.is_valid() {
                    self.bounding_boxes.push(bounding_box);
                }
                None
            }
            (Some(mut bounding_box), false, false, false) => {
                // move the box corner
                if let Some(position) = mouse_position {
                    bounding_box.set_opposing_corner(position);
                }
                Some(bounding_box)
            }
            (None, true, false, false) => {
                // create a new box
                mouse_position
                    .map(|position| BoundingBox::new(position, position, self.selected_class))
            }
            (None, false, true, false) => {
                // select a box for editing
                mouse_position.and_then(|position| {
                    if let Some((index, _)) = self
                        .bounding_boxes
                        .iter()
                        .enumerate()
                        .find(|(_, bounding_box)| bounding_box.has_corner_at(position))
                    {
                        let mut bbox = self.bounding_boxes.remove(index);
                        bbox.prepare_for_corner_move(position);
                        return Some(bbox);
                    }
                    None
                })
            }

            (_, _, _, _) => None,
        };
        if let Some(bbox) = editing_bounding_box {
            let _ = self.box_in_editing.insert(bbox);
        }
    }
}

impl<'a> Widget for BoundingBoxAnnotator<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let response = Plot::new(self.id)
            .data_aspect(1.)
            .view_aspect(640. / 480.)
            .show_axes([false, false])
            .show_grid([false, false])
            .set_margin_fraction(Vec2::splat(0.1))
            .auto_bounds_x()
            .auto_bounds_y()
            .show_background(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.image(PlotImage::new(
                    &self.texture_handle,
                    PlotPoint::new(320., 240.),
                    Vec2::new(640., 480.),
                ));
                self.bounding_boxes
                    .iter()
                    .chain(self.box_in_editing.iter())
                    .filter(|bbox| bbox.is_valid())
                    .for_each(|bbox| {
                        let polygon: Polygon = bbox.into();
                        plot_ui.polygon(
                            polygon
                                .fill_color(bbox.class.color())
                                .stroke(Stroke::new(1.0, bbox.class.color().to_opaque())),
                        );
                        plot_ui.text(
                            Text::new(
                                bbox.top_left(),
                                RichText::new(format!("{:?}", bbox.class))
                                    .color(Color32::GRAY)
                                    .strong()
                                    .size(18.)
                                    .background_color(bbox.class.color().to_opaque()),
                            )
                            .anchor(Align2::LEFT_BOTTOM),
                        )
                    });
            });
        self.handle_bounding_box_input(&response, ui);

        if let (Some(position), None) = (response.response.hover_pos(), self.box_in_editing) {
            let position = response.transform.value_from_position(position);
            if let Some(bbox) = self
                .bounding_boxes
                .iter()
                .find(|bbox| bbox.has_corner_at(position))
            {
                let corner = bbox.get_closest_corner(position);
                let corner_screen = response.transform.position_from_point(&corner);
                let radius = 5.0 * response.transform.dpos_dvalue_x();
                let painter = ui.painter();
                painter.circle_stroke(
                    corner_screen,
                    radius as f32,
                    Stroke::new(2.0, Color32::GRAY),
                );
            }
        }

        response.response
    }
}
