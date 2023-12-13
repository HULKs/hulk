use eframe::{
    egui::{Event, Id, Key, PointerButton, Response, RichText, Ui, Widget},
    emath::{Align2, Vec2b},
    epaint::{Color32, Stroke, TextureHandle, Vec2},
};
use egui_plot::{Plot, PlotBounds, PlotImage, PlotPoint, PlotResponse, PlotUi, Polygon, Text};
use std::hash::Hash;

use crate::{boundingbox::BoundingBox, classes::Classes};

pub struct BoundingBoxAnnotator<'a> {
    id: Id,
    texture_handle: TextureHandle,
    selected_class: &'a mut Classes,
    bounding_boxes: &'a mut Vec<BoundingBox>,
    box_in_editing: &'a mut Option<BoundingBox>,
}

impl<'a> BoundingBoxAnnotator<'a> {
    pub fn new(
        id_source: impl Hash,
        image: TextureHandle,
        bounding_boxes: &'a mut Vec<BoundingBox>,
        box_in_editing: &'a mut Option<BoundingBox>,
        selected_class: &'a mut Classes,
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
            ui.input(|i| i.key_pressed(Key::Q)),
            response.response.clicked_by(PointerButton::Secondary)
                || ui.input(|i| i.key_pressed(Key::Escape)),
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
            (Some(mut bounding_box), b_pressed, q_pressed, false) if b_pressed || q_pressed => {
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
                bounding_box.class = *self.selected_class;
                Some(bounding_box)
            }
            (None, true, false, false) => {
                // create a new box
                mouse_position
                    .map(|position| BoundingBox::new(position, position, *self.selected_class))
            }
            (None, false, true, false) => {
                // select a box for editing
                mouse_position.and_then(|position| {
                    if let Some((index, _)) = self.bounding_boxes.iter().enumerate().min_by(
                        |(_, bounding_box1), (_, bounding_box2)| {
                            bounding_box1
                                .closest_corner_distance_sq(position)
                                .total_cmp(&bounding_box2.closest_corner_distance_sq(position))
                        },
                    ) {
                        let mut bbox = self.bounding_boxes.remove(index);
                        bbox.prepare_for_corner_move(position);
                        *self.selected_class = bbox.class;
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
            .show_background(true)
            .allow_scroll(false)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .show(ui, |plot_ui| {
                zoom_on_scroll_wheel(plot_ui);
                focus_when_e_held_down(plot_ui);

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

fn zoom_on_scroll_wheel(plot_ui: &mut PlotUi) {
    let scroll_delta = plot_ui.ctx().input(|i| {
        i.events.iter().find_map(|e| match e {
            Event::MouseWheel { delta, .. } => Some(*delta),
            _ => None,
        })
    });

    if let Some(scroll_delta) = scroll_delta {
        let zoom_factor = ((scroll_delta.x + scroll_delta.y) / 10.0).exp();
        zoom_plot(plot_ui, zoom_factor);
    }
}

fn focus_when_e_held_down(plot_ui: &mut PlotUi) {
    if let Some(pressed) = plot_ui.ctx().input(|i| {
        i.events.iter().find_map(|e| match e {
            Event::Key {
                key: Key::E,
                repeat: false,
                pressed,
                ..
            } => Some(*pressed),
            _ => None,
        })
    }) {
        if !pressed {
            plot_ui.set_auto_bounds(Vec2b::TRUE);
            return;
        }
        zoom_plot(plot_ui, 5.0);
    }
}

fn zoom_plot(plot_ui: &mut PlotUi, zoom_factor: f32) {
    let Some(zoom_center) = plot_ui.pointer_coordinate() else {
        return;
    };

    let old_bounds = plot_ui.plot_bounds();
    let new_bounds = zoom_bounds(old_bounds, Vec2::splat(zoom_factor), zoom_center);

    plot_ui.set_plot_bounds(new_bounds);
}

/// Same as PlotTransform::zoom() which is inaccessible from here.
fn zoom_bounds(bounds: PlotBounds, zoom_factor: Vec2, zoom_center: PlotPoint) -> PlotBounds {
    let mut min = bounds.min();
    let mut max = bounds.max();

    min[0] = zoom_center.x + (min[0] - zoom_center.x) / (zoom_factor.x as f64);
    max[0] = zoom_center.x + (max[0] - zoom_center.x) / (zoom_factor.x as f64);
    min[1] = zoom_center.y + (min[1] - zoom_center.y) / (zoom_factor.y as f64);
    max[1] = zoom_center.y + (max[1] - zoom_center.y) / (zoom_factor.y as f64);

    PlotBounds::from_min_max(min, max)
}
